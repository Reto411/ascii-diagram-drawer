//! Parser: `.ad` text → [`Document`] / [`Diagram`], per the normative grammar in
//! [diagram-file-format.md](../../../doc/arc42/12-concepts/diagram-file-format.md).
//!
//! Lenient in, strict out: whitespace runs, CRLF endings and `#` comments are tolerated;
//! every known statement must be well-formed. Unknown statements are preserved verbatim
//! only when the file's minor version is higher than this reader's (forward compatibility,
//! QS7 across minor upgrades).

use std::collections::BTreeMap;

use diagram_model::Diagram;
use diagram_model::anchors::{AnchorRef, ConnectorEndpoint};
use diagram_model::color::{ColorRole, Rgb};
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::invariants::ModelError;
use diagram_model::objects::{
    ConnectorInstance, ConnectorStyle, Object, ObjectId, ShapeInstance, TextInstance, TipStyle,
};

use crate::error::ParseError;

/// The major format version this reader implements.
const MAJOR: u32 = 1;
/// The minor version this reader implements (1.1 adds the `colors` statement, CR-009);
/// files with a higher minor are read in forward-compat mode.
const MINOR: u32 = 1;

/// A parsed `.ad` file: the diagram content plus unknown statements preserved verbatim,
/// so re-saving after a minor-version upgrade loses nothing (QS7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The diagram content (scale + objects in file order).
    pub diagram: Diagram,
    /// Statement slots in original file position; `Object` indexes are into the object list
    /// at parse time and become stale if the diagram is mutated after loading.
    pub(crate) slots: Vec<Slot>,
    /// The source file's header version, re-emitted on save so preserved unknown statements
    /// stay legal (forward-compat round trip, QS7).
    pub(crate) version: (u32, u32),
}

/// One statement slot of the original file, preserving its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Slot {
    /// A known object statement; index into [`Document::diagram`]'s object list at parse time.
    Object(usize),
    /// An unknown statement preserved verbatim (forward-compat mode only).
    Unknown(String),
}

/// A known object statement parsed but not yet applied to the model (two-phase parse, S-043):
/// application is deferred until every statement is read, so a connector's endpoint may
/// reference a shape declared later in the file (forward reference — legal per the format).
#[derive(Debug, Clone)]
enum PendingObject {
    Shape(ShapeInstance),
    Text(TextInstance),
    Connect(ConnectorInstance),
}

impl Document {
    /// Number of preserved unknown statements.
    #[must_use]
    pub fn unknown_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, Slot::Unknown(_)))
            .count()
    }
}

/// Parses `.ad` text into a [`Diagram`].
///
/// # Errors
/// [`ParseError`] with line/column on any malformed input (bad header, bad version,
/// truncated statement, invalid integer/escape/anchor-ref, or a model invariant violation).
pub fn parse(input: &str) -> Result<Diagram, ParseError> {
    parse_document(input).map(|document| document.diagram)
}

/// Parses `.ad` text into a [`Document`], preserving unknown statements verbatim when the
/// file's minor version is higher than this reader's.
///
/// # Errors
/// [`ParseError`] with line/column on any malformed input; an unsupported major version
/// rejects the whole file.
pub fn parse_document(input: &str) -> Result<Document, ParseError> {
    let mut diagram = Diagram::new(); // receives scale/colors while streaming
    let mut pending: Vec<(usize, usize, PendingObject)> = Vec::new(); // (line, column, statement)
    let mut slots: Vec<Slot> = Vec::new();
    let mut header_seen = false;
    let mut version = (MAJOR, MINOR);
    let mut scale_seen = false;
    let mut colors_seen = false;

    for (index, raw_line) in input.lines().enumerate() {
        let line_no = index.saturating_add(1);
        let tokens = tokenize(raw_line, line_no)?;
        if tokens.is_empty() {
            continue; // blank line or comment-only line: decoration, no model information
        }
        let Some(first) = tokens.first() else {
            continue;
        };
        if !header_seen {
            version = parse_header(&tokens, line_no)?;
            header_seen = true;
            continue;
        }
        match first.as_str() {
            "scale" => parse_scale(
                &tokens,
                line_no,
                &mut scale_seen,
                !pending.is_empty(),
                &mut diagram,
            )?,
            "colors" => parse_colors(
                &tokens,
                line_no,
                &mut colors_seen,
                !pending.is_empty(),
                &mut diagram,
            )?,
            "shape" => {
                let mut cursor = Cursor::new(&tokens);
                cursor.next(); // consume the matched "shape" keyword
                let shape = parse_shape(&mut cursor, line_no)?;
                slots.push(Slot::Object(pending.len()));
                pending.push((line_no, first.column, PendingObject::Shape(shape)));
            }
            "text" => {
                let mut cursor = Cursor::new(&tokens);
                cursor.next(); // consume the matched "text" keyword
                let text = parse_text(&mut cursor, line_no)?;
                slots.push(Slot::Object(pending.len()));
                pending.push((line_no, first.column, PendingObject::Text(text)));
            }
            "connect" => {
                let mut cursor = Cursor::new(&tokens);
                cursor.next(); // consume the matched "connect" keyword
                let connector = parse_connect(&mut cursor, line_no)?;
                slots.push(Slot::Object(pending.len()));
                pending.push((line_no, first.column, PendingObject::Connect(connector)));
            }
            other => {
                if version.1 > MINOR {
                    // A run of trailing `\r` is line-break decoration (CRLF normalization):
                    // re-reading a preserved line strips one more `\r`, so keeping any
                    // would break the byte-for-byte round trip (B-008). Output is LF-only.
                    slots.push(Slot::Unknown(raw_line.trim_end_matches('\r').to_owned()));
                } else {
                    return Err(ParseError::new(
                        line_no,
                        first.column,
                        format!(
                            "unknown statement '{other}' — version 1.0 files may only contain known statements"
                        ),
                    ));
                }
            }
        }
    }

    if !header_seen {
        return Err(ParseError::new(
            1,
            1,
            "missing header — the first statement must be 'ad <major>.<minor>'",
        ));
    }

    // Two-phase application (S-043 / ADR 018): install every object at once so a connector's
    // endpoint may reference a shape declared later in the file (forward reference). File
    // order (= z-order) is preserved; all aggregate invariants are checked against the full
    // list by the model.
    let objects: Vec<Object> = pending
        .iter()
        .map(|(_, _, statement)| match statement {
            PendingObject::Shape(shape) => Object::Shape(shape.clone()),
            PendingObject::Text(text) => Object::Text(text.clone()),
            PendingObject::Connect(connector) => Object::Connector(connector.clone()),
        })
        .collect();
    let diagram = Diagram::from_parts(objects, diagram.scale(), diagram.colors()).map_err(|e| {
        let (line_no, column) = error_position(&e, &pending);
        model_error(e, line_no, column)
    })?;
    Ok(Document {
        diagram,
        slots,
        version,
    })
}

/// Maps a deferred model-invariant error back to the statement that caused it (S-043):
/// id-carrying errors point at the declaring statement; reference errors at the connector
/// that made the reference.
fn error_position(error: &ModelError, pending: &[(usize, usize, PendingObject)]) -> (usize, usize) {
    let declared = |id: &ObjectId| {
        pending.iter().find_map(|(line, col, statement)| {
            (pending_id(statement) == id).then_some((*line, *col))
        })
    };
    let referencing = |id: &ObjectId| {
        pending
            .iter()
            .find_map(|(line, col, statement)| match statement {
                PendingObject::Connect(connector)
                    if connector.from.object() == Some(id) || connector.to.object() == Some(id) =>
                {
                    Some((*line, *col))
                }
                _ => None,
            })
    };
    match error {
        ModelError::DuplicateId(id) => {
            // Point at the *second* declaration — the statement that failed.
            let mut seen_once = false;
            pending
                .iter()
                .find_map(|(line, col, statement)| {
                    if pending_id(statement) == id {
                        if seen_once {
                            return Some((*line, *col));
                        }
                        seen_once = true;
                    }
                    None
                })
                .unwrap_or((1, 1))
        }
        ModelError::ZeroLengthConnector(id) => declared(id).unwrap_or((1, 1)),
        ModelError::UnknownObject(id) | ModelError::NotAShape(id) => {
            referencing(id).or_else(|| declared(id)).unwrap_or((1, 1))
        }
        _ => (1, 1), // e.g. ScaleTooSmall — already validated while streaming
    }
}

/// The id of a pending object statement.
fn pending_id(statement: &PendingObject) -> &ObjectId {
    match statement {
        PendingObject::Shape(shape) => &shape.id,
        PendingObject::Text(text) => &text.id,
        PendingObject::Connect(connector) => &connector.id,
    }
}

/// Splits one physical line into tokens; a `#` outside quotes starts a comment to end of line.
fn tokenize(raw: &str, line_no: usize) -> Result<Vec<Token>, ParseError> {
    let mut stream = CharStream::new(raw);
    let mut tokens = Vec::new();
    loop {
        let Some((col, c)) = stream.next() else {
            return Ok(tokens);
        };
        match c {
            // `\r` is whitespace, not word content: a lone trailing `\r` (not part of a
            // CRLF pair — `lines()` keeps it) must not be absorbed into the last token,
            // which would mask keywords like `colors` and silently preserve them as
            // unknown statements (B-009).
            ' ' | '\t' | '\r' => {}
            '#' => return Ok(tokens),
            '"' => tokens.push(read_string(&mut stream, line_no, col)?),
            // A comma is its own token (list separator — e.g. `connect … via 3 4, 5 6`).
            ',' => {
                tokens.push(Token::word(",".to_owned(), col));
            }
            _ => tokens.push(read_word(&mut stream, c, col)),
        }
    }
}

/// Reads the remainder of a bare word (already started with `first`); words end at
/// whitespace, a quote, or a comment start.
fn read_word(stream: &mut CharStream, first: char, column: usize) -> Token {
    let mut text = String::new();
    text.push(first);
    while let Some(next) = stream.peek_char() {
        if matches!(next, ' ' | '\t' | '\r' | '"' | '#' | ',') {
            break;
        }
        if let Some((_, c)) = stream.next() {
            text.push(c);
        }
    }
    Token::word(text, column)
}

/// Reads a quoted string starting at the opening quote in `column`; resolves the v1 escapes.
fn read_string(
    stream: &mut CharStream,
    line_no: usize,
    column: usize,
) -> Result<Token, ParseError> {
    let mut value = String::new();
    loop {
        let Some((col, c)) = stream.next() else {
            return Err(ParseError::new(
                line_no,
                column,
                "unterminated string — no closing quote on this line",
            ));
        };
        match c {
            '"' => return Ok(Token::quoted(value, column)),
            '\\' => {
                let Some((_, escape)) = stream.next() else {
                    return Err(ParseError::new(
                        line_no,
                        col,
                        "unterminated escape — backslash at end of line",
                    ));
                };
                match escape {
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    'n' => value.push('\n'),
                    other => {
                        // Report the backslash's column: that is where the invalid escape starts.
                        return Err(ParseError::new(
                            line_no,
                            col,
                            format!(
                                "invalid escape '\\{other}' — only \\\" \\\\ and \\n are valid"
                            ),
                        ));
                    }
                }
            }
            other => value.push(other),
        }
    }
}

/// Parses the header statement `ad <major>.<minor>` and returns its `(major, minor)` version.
fn parse_header(tokens: &[Token], line_no: usize) -> Result<(u32, u32), ParseError> {
    let (kind, after_kind) = tokens.split_first().ok_or_else(|| {
        ParseError::new(
            line_no,
            1,
            "missing header — the first statement must be 'ad <major>.<minor>'",
        )
    })?;
    if kind.as_str() != "ad" {
        return Err(ParseError::new(
            line_no,
            kind.column,
            format!("expected header keyword 'ad', found '{}'", kind.as_str()),
        ));
    }
    let (version_tok, after_version) = split(after_kind, line_no)?;
    let (major, minor, major_col) = parse_version(version_tok, after_version, line_no)?;
    if major != MAJOR {
        return Err(ParseError::new(
            line_no,
            major_col,
            format!(
                "unsupported major version {major} — this reader supports major version {MAJOR}"
            ),
        ));
    }
    Ok((major, minor))
}

/// Parses the version part of a header; accepts the canonical single-token form `major.minor`
/// and, leniently, the three-token form `major . minor`.
fn parse_version(
    first: &Token,
    rest: &[Token],
    line_no: usize,
) -> Result<(u32, u32, usize), ParseError> {
    if let Some((maj_text, min_text)) = first.as_str().split_once('.') {
        // Canonical form: one "major.minor" token.
        let major = parse_int_text(maj_text, line_no, first.column)?;
        let minor_col = first
            .column
            .saturating_add(maj_text.chars().count().saturating_add(1));
        let minor = parse_int_text(min_text, line_no, minor_col)?;
        expect_no_leftover(rest, line_no, "the header version")?;
        return Ok((major, minor, first.column));
    }
    // Lenient form: three tokens `major` `.` `minor`.
    let major = parse_int(first, line_no)?;
    let (dot, after_dot) = split(rest, line_no)?;
    if dot.as_str() != "." {
        return Err(ParseError::new(
            line_no,
            dot.column,
            format!("expected '.' in version number, found '{}'", dot.as_str()),
        ));
    }
    let (minor_tok, leftover) = split(after_dot, line_no)?;
    let minor = parse_int(minor_tok, line_no)?;
    expect_no_leftover(leftover, line_no, "the header version")?;
    Ok((major, minor, first.column))
}

/// Errors when tokens remain after a complete sub-statement.
fn expect_no_leftover(rest: &[Token], line_no: usize, what: &str) -> Result<(), ParseError> {
    if let Some(extra) = rest.first() {
        return Err(ParseError::new(
            line_no,
            extra.column,
            format!("unexpected token '{}' after {what}", extra.as_str()),
        ));
    }
    Ok(())
}

/// Splits off the first token of `rest`, erroring when the statement is truncated.
fn split(rest: &[Token], line_no: usize) -> Result<(&Token, &[Token]), ParseError> {
    rest.split_first().ok_or_else(|| {
        ParseError::new(
            line_no,
            1,
            "truncated header — expected 'ad <major>.<minor>'",
        )
    })
}

/// Parses a canonical decimal integer token (no sign, no leading zeros, u32 range).
fn parse_int(token: &Token, line_no: usize) -> Result<u32, ParseError> {
    parse_int_text(token.as_str(), line_no, token.column)
}

/// Parses a canonical decimal integer from raw text at `column` (no sign, no leading zeros, u32 range).
fn parse_int_text(text: &str, line_no: usize, column: usize) -> Result<u32, ParseError> {
    if text.is_empty() {
        return Err(ParseError::new(line_no, column, "empty integer"));
    }
    if matches!(text.chars().next(), Some('-' | '+')) {
        return Err(ParseError::new(
            line_no,
            column,
            format!("unexpected sign in integer '{text}' — integers are unsigned"),
        ));
    }
    if !text.chars().all(|c| c.is_ascii_digit()) {
        return Err(ParseError::new(
            line_no,
            column,
            format!("'{text}' is not a decimal integer"),
        ));
    }
    if text.len() > 1 && text.starts_with('0') {
        return Err(ParseError::new(
            line_no,
            column,
            format!("leading zeros are not allowed in '{text}' — integers are canonical decimal"),
        ));
    }
    text.parse::<u32>()
        .map_err(|_| ParseError::new(line_no, column, format!("'{text}' does not fit a u32")))
}

/// Parses `scale <int>`; at most one per file and it must precede every object statement.
fn parse_scale(
    tokens: &[Token],
    line_no: usize,
    scale_seen: &mut bool,
    objects_seen: bool,
    diagram: &mut Diagram,
) -> Result<(), ParseError> {
    if *scale_seen {
        let col = tokens.first().map_or(1, |t| t.column);
        return Err(ParseError::new(
            line_no,
            col,
            "duplicate 'scale' statement — at most one per file",
        ));
    }
    if objects_seen {
        let col = tokens.first().map_or(1, |t| t.column);
        return Err(ParseError::new(
            line_no,
            col,
            "'scale' must precede every object statement",
        ));
    }
    let mut cursor = Cursor::new(tokens);
    cursor.next(); // consume the matched "scale" keyword
    let value = cursor.expect_int("the scale value", line_no)?;
    cursor.expect_end("scale statement", line_no)?;
    diagram
        .set_scale(value)
        .map_err(|e| model_error(e, line_no, 1))?;
    *scale_seen = true;
    Ok(())
}

/// Parses `colors [<role> "#rrggbb"]+` (CR-009); at most one per file and it must
/// precede every object statement, like `scale`. Roles are bare words (`background`,
/// `fill`, `lines`), hex values quoted strings; each role at most once per statement.
fn parse_colors(
    tokens: &[Token],
    line_no: usize,
    colors_seen: &mut bool,
    objects_seen: bool,
    diagram: &mut Diagram,
) -> Result<(), ParseError> {
    let col = |tokens: &[Token]| tokens.first().map_or(1, |t| t.column);
    if *colors_seen {
        return Err(ParseError::new(
            line_no,
            col(tokens),
            "duplicate 'colors' statement — at most one per file",
        ));
    }
    if objects_seen {
        return Err(ParseError::new(
            line_no,
            col(tokens),
            "'colors' must precede every object statement",
        ));
    }
    let mut cursor = Cursor::new(tokens);
    cursor.next(); // consume the matched "colors" keyword
    let mut any = false;
    let mut seen_roles: Vec<ColorRole> = Vec::new();
    while let Some(role_token) = cursor.peek() {
        // Copy the token data out: peek borrows the cursor, which the next() calls below need.
        let role_text = role_token.as_str().to_owned();
        let role_col = role_token.column;
        let role = ColorRole::parse(&role_text).ok_or_else(|| {
            ParseError::new(
                line_no,
                role_col,
                format!(
                    "unknown color role '{role_text}' — expected 'background', 'fill' or 'lines'"
                ),
            )
        })?;
        if seen_roles.contains(&role) {
            return Err(ParseError::new(
                line_no,
                role_col,
                format!("duplicate color role '{role_text}' in one 'colors' statement"),
            ));
        }
        cursor.next(); // consume the role word
        let Some(hex_token) = cursor.peek() else {
            return Err(ParseError::new(
                line_no,
                1,
                format!("statement is truncated — expected the {role_text} color value"),
            ));
        };
        // Same: copy out the fields the error paths need before consuming.
        let (hex_quoted, hex_col, hex_text) = (
            hex_token.quoted,
            hex_token.column,
            hex_token.as_str().to_owned(),
        );
        if !hex_quoted {
            return Err(ParseError::new(
                line_no,
                hex_col,
                format!(
                    "expected a quoted \"#rrggbb\" value for the {role_text} color, found bare word '{hex_text}'"
                ),
            ));
        }
        cursor.next(); // consume the quoted hex value
        let rgb = Rgb::from_hex(&hex_text).map_err(|_| {
            ParseError::new(
                line_no,
                hex_col,
                format!(
                    "'{hex_text}' is not a valid color — expected \"#rrggbb\" (six hex digits after '#')"
                ),
            )
        })?;
        diagram.set_color(role, Some(rgb));
        seen_roles.push(role);
        any = true;
    }
    if !any {
        return Err(ParseError::new(
            line_no,
            col(tokens),
            "'colors' statement is empty — expected at least one <role> \"#rrggbb\" pair",
        ));
    }
    *colors_seen = true;
    Ok(())
}

/// Parses `shape <type-id> "<label>" as <id> at <x> <y> w <width> h <height>` (keyword consumed).
fn parse_shape(cursor: &mut Cursor, line_no: usize) -> Result<ShapeInstance, ParseError> {
    let type_id = cursor.expect_word("a type-id", line_no)?;
    let label = cursor.expect_quoted("the label", line_no)?;
    cursor.expect_keyword("as", line_no)?;
    let id = cursor.expect_word("an id", line_no)?;
    cursor.expect_keyword("at", line_no)?;
    let x = cursor.expect_int("x coordinate", line_no)?;
    let y = cursor.expect_int("y coordinate", line_no)?;
    cursor.expect_keyword("w", line_no)?;
    let width = cursor.expect_int("width", line_no)?;
    cursor.expect_keyword("h", line_no)?;
    let height = cursor.expect_int("height", line_no)?;
    // Optional region clauses (FR19 / CR-016): `region <name> "<content>"` — zero or
    // more, in any order; names are unique per statement.
    let mut regions: BTreeMap<String, String> = BTreeMap::new();
    while let Some(tok) = cursor.peek() {
        if tok.as_str() != "region" {
            break;
        }
        cursor.next();
        let name = cursor.expect_word("a region name", line_no)?;
        let content = cursor.expect_quoted("the region content", line_no)?;
        if regions.insert(name.clone(), content).is_some() {
            return Err(ParseError::new(
                line_no,
                1,
                format!("region '{name}' is declared more than once in this shape statement"),
            ));
        }
    }
    cursor.expect_end("shape statement", line_no)?;

    let size = CellSize::new(width, height).map_err(|e| model_error(e, line_no, 1))?;
    let mut shape = ShapeInstance::new(
        ObjectId::new(&id).map_err(|e| model_error(e, line_no, 1))?,
        &type_id,
        &label,
        CellPos::new(x, y),
        size,
    )
    .map_err(|e| model_error(e, line_no, 1))?;
    shape.regions = regions;
    Ok(shape)
}

/// Parses `text as <id> "<content>" at <x> <y>` (keyword consumed).
fn parse_text(cursor: &mut Cursor, line_no: usize) -> Result<TextInstance, ParseError> {
    cursor.expect_keyword("as", line_no)?;
    let id = cursor.expect_word("an id", line_no)?;
    let content = cursor.expect_quoted("the text content", line_no)?;
    cursor.expect_keyword("at", line_no)?;
    let x = cursor.expect_int("x coordinate", line_no)?;
    let y = cursor.expect_int("y coordinate", line_no)?;
    cursor.expect_end("text statement", line_no)?;

    TextInstance::new(
        ObjectId::new(&id).map_err(|e| model_error(e, line_no, 1))?,
        CellPos::new(x, y),
        &content,
    )
    .map_err(|e| model_error(e, line_no, 1))
}

/// Parses `connect as <id> from <endpoint> to <endpoint> [dashed]` (keyword consumed).
/// Each endpoint is either `<id>.<anchor>` (attached) or `<x> <y>` (free — CR-010 / ADR 013 v6).
fn parse_connect(cursor: &mut Cursor, line_no: usize) -> Result<ConnectorInstance, ParseError> {
    cursor.expect_keyword("as", line_no)?;
    let id = cursor.expect_word("an id", line_no)?;
    cursor.expect_keyword("from", line_no)?;
    let from = parse_endpoint(cursor, line_no)?;
    cursor.expect_keyword("to", line_no)?;
    let to = parse_endpoint(cursor, line_no)?;
    // Optional clauses in fixed order (CR-007/CR-008):
    // `[dashed] [tip <src> <tgt>] [via <x> <y>, …]`.
    let style = match cursor.peek() {
        Some(dashed) if dashed.as_str() == "dashed" => {
            cursor.next();
            ConnectorStyle::Dashed
        }
        _ => ConnectorStyle::Solid,
    };
    let tips = match cursor.peek() {
        Some(tip) if tip.as_str() == "tip" => {
            cursor.next();
            (
                parse_tip_style(cursor, line_no)?,
                parse_tip_style(cursor, line_no)?,
            )
        }
        _ => (TipStyle::Bare, TipStyle::HollowArrow), // defaults — not serialized
    };
    let bends = match cursor.peek() {
        Some(via) if via.as_str() == "via" => {
            cursor.next();
            parse_via_list(cursor, line_no)?
        }
        Some(extra) => {
            return Err(ParseError::new(
                line_no,
                extra.column,
                format!(
                    "unexpected token '{}' — only 'via …' may follow here (clause order: [dashed] [tip <src> <tgt>] [via …])",
                    extra.as_str()
                ),
            ));
        }
        None => Vec::new(),
    };
    let object_id = ObjectId::new(&id).map_err(|e| model_error(e, line_no, 1))?;
    Ok(ConnectorInstance::new(object_id, from, to, style)
        .with_bends(bends)
        .with_tips(tips.0, tips.1))
}

/// Parses one connector endpoint (CR-010 / ADR 013 v6): either `<id>.<anchor>` — attached,
/// containing a dot — or `<x> <y>` — free at an explicit model-space cell, two bare
/// non-negative integers. The token shape disambiguates: object ids and anchor names are
/// single tokens, so a dotted token can only be an anchor-ref.
fn parse_endpoint(cursor: &mut Cursor, line_no: usize) -> Result<ConnectorEndpoint, ParseError> {
    let token = cursor.expect_word("an endpoint ('<id>.<anchor>' or '<x> <y>')", line_no)?;
    if token.contains('.') {
        return Ok(ConnectorEndpoint::Attached(parse_anchor_ref(
            token, line_no,
        )?));
    }
    // No dot — the first token is the x coordinate of a free endpoint.
    let x = parse_int_text(&token, line_no, 1).map_err(|mut error| {
        error.message = format!(
            "free endpoint x coordinate: {} (an endpoint is '<id>.<anchor>' or '<x> <y>')",
            error.message
        );
        error
    })?;
    let y = cursor.expect_int("the free endpoint's y coordinate", line_no)?;
    Ok(ConnectorEndpoint::Free(CellPos::new(x, y)))
}

/// Parses the optional `via <x> <y>, …` manual-bend waypoint list (CR-007 / ADR 013 v4).
/// Waypoints are bare non-negative cell coordinates in model space, comma-separated.
fn parse_via_list(cursor: &mut Cursor, line_no: usize) -> Result<Vec<CellPos>, ParseError> {
    let mut bends = Vec::new();
    loop {
        let x = cursor.expect_int("a via x coordinate", line_no)?;
        let y = cursor.expect_int("a via y coordinate", line_no)?;
        bends.push(CellPos::new(x, y));
        match cursor.peek() {
            Some(comma) if comma.as_str() == "," => {
                cursor.next();
            }
            Some(extra) => {
                return Err(ParseError::new(
                    line_no,
                    extra.column,
                    format!(
                        "unexpected token '{}' — a via list is '<x> <y>' pairs separated by ','",
                        extra.as_str()
                    ),
                ));
            }
            None => break,
        }
    }
    Ok(bends)
}

/// One tip-style keyword of a `tip <src> <tgt>` clause (CR-008): `none`, `arrow`,
/// `rhombus-open` or `rhombus-filled`.
fn parse_tip_style(cursor: &mut Cursor, line_no: usize) -> Result<TipStyle, ParseError> {
    let Some(token) = cursor.next() else {
        return Err(ParseError::new(
            line_no,
            1,
            "statement is truncated — expected a tip style (one of none, arrow, rhombus-open, rhombus-filled)",
        ));
    };
    match token.as_str() {
        "none" => Ok(TipStyle::Bare),
        "arrow" => Ok(TipStyle::HollowArrow),
        "rhombus-open" => Ok(TipStyle::EmptyRhombus),
        "rhombus-filled" => Ok(TipStyle::FilledRhombus),
        other => Err(ParseError::new(
            line_no,
            token.column,
            format!(
                "unknown tip style '{other}' — expected one of none, arrow, rhombus-open, rhombus-filled"
            ),
        )),
    }
}

/// Parses `<id>.<anchor>` split at the LAST dot (ids and anchors may themselves contain dots).
fn parse_anchor_ref(ref_text: String, line_no: usize) -> Result<AnchorRef, ParseError> {
    let Some(dot) = ref_text.rfind('.') else {
        return Err(ParseError::new(
            line_no,
            1,
            format!("anchor-ref '{ref_text}' must be '<id>.<anchor>' — no dot found"),
        ));
    };
    let (object_id, rest) = ref_text.split_at(dot);
    let Some(anchor) = rest.strip_prefix('.') else {
        return Err(ParseError::new(
            line_no,
            1,
            format!("anchor-ref '{ref_text}' must be '<id>.<anchor>' — no dot found"),
        ));
    };
    if object_id.is_empty() || anchor.is_empty() {
        return Err(ParseError::new(
            line_no,
            1,
            format!("anchor-ref '{ref_text}' — id and anchor must both be non-empty"),
        ));
    }
    let object = ObjectId::new(object_id).map_err(|e| model_error(e, line_no, 1))?;
    AnchorRef::new(object, anchor).map_err(|e| model_error(e, line_no, 1))
}

/// Converts a model invariant violation into a parse error at the statement's position.
fn model_error(error: ModelError, line_no: usize, column: usize) -> ParseError {
    ParseError::new(line_no, column, error.to_string())
}

/// A lexical token with its 1-based starting column.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    /// 1-based character column where the token starts.
    column: usize,
    /// Whether the token was a quoted string.
    quoted: bool,
    /// The token text (string values with escapes already resolved).
    text: String,
}

impl Token {
    /// The token text.
    #[must_use]
    fn as_str(&self) -> &str {
        &self.text
    }

    /// Builds a quoted-string token (value with escapes resolved).
    #[must_use]
    fn quoted(text: String, column: usize) -> Self {
        Self {
            column,
            text,
            quoted: true,
        }
    }

    /// Builds a bare-word token.
    #[must_use]
    fn word(text: String, column: usize) -> Self {
        Self {
            column,
            text,
            quoted: false,
        }
    }
}

/// A read cursor over a statement's token sequence.
struct Cursor<'a> {
    pos: usize,
    tokens: &'a [Token],
}

impl<'a> Cursor<'a> {
    /// Asserts the statement is complete (no trailing tokens).
    fn expect_end(&self, statement: &str, line_no: usize) -> Result<(), ParseError> {
        if let Some(extra) = self.peek() {
            return Err(ParseError::new(
                line_no,
                extra.column,
                format!(
                    "unexpected token '{}' after complete {statement}",
                    extra.as_str()
                ),
            ));
        }
        Ok(())
    }

    /// An integer expected next (canonical decimal, u32).
    fn expect_int(&mut self, what: &str, line_no: usize) -> Result<u32, ParseError> {
        let Some(token) = self.next() else {
            return Err(ParseError::new(
                line_no,
                1,
                format!("statement is truncated — expected {what}"),
            ));
        };
        match parse_int(token, line_no) {
            Ok(value) => Ok(value),
            Err(mut error) => {
                error.message = format!("expected {what}: {}", error.message);
                Err(error)
            }
        }
    }

    /// The keyword expected next, or an error naming what was found instead.
    fn expect_keyword(&mut self, keyword: &str, line_no: usize) -> Result<(), ParseError> {
        let Some(token) = self.next() else {
            return Err(ParseError::new(
                line_no,
                1,
                format!("statement is truncated — expected '{keyword}'"),
            ));
        };
        if token.as_str() != keyword {
            return Err(ParseError::new(
                line_no,
                token.column,
                format!("expected '{keyword}', found '{}'", token.as_str()),
            ));
        }
        Ok(())
    }

    /// A bare word expected next (ids, type-ids, anchor-refs are never quoted).
    fn expect_word(&mut self, what: &str, line_no: usize) -> Result<String, ParseError> {
        let Some(token) = self.next() else {
            return Err(ParseError::new(
                line_no,
                1,
                format!("statement is truncated — expected {what}"),
            ));
        };
        if token.quoted {
            return Err(ParseError::new(
                line_no,
                token.column,
                format!("{what} must be a bare word, not a quoted string"),
            ));
        }
        Ok(token.text.clone())
    }

    /// A quoted string expected next.
    fn expect_quoted(&mut self, what: &str, line_no: usize) -> Result<String, ParseError> {
        let Some(token) = self.next() else {
            return Err(ParseError::new(
                line_no,
                1,
                format!("statement is truncated — expected {what}"),
            ));
        };
        if !token.quoted {
            return Err(ParseError::new(
                line_no,
                token.column,
                format!(
                    "expected a quoted string for {what}, found bare word '{}'",
                    token.as_str()
                ),
            ));
        }
        Ok(token.text.clone())
    }

    /// Starts a cursor at the beginning of `tokens`.
    #[must_use]
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Consumes and returns the next token.
    fn next(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos)?;
        self.pos = self.pos.saturating_add(1);
        Some(token)
    }

    /// The token at the cursor without consuming it.
    #[must_use]
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
}

/// A character stream with 1-based column tracking.
struct CharStream<'a> {
    /// 1-based column of the next character to be returned.
    col: usize,
    inner: core::iter::Peekable<core::str::Chars<'a>>,
}

impl<'a> CharStream<'a> {
    /// Wraps `raw` for sequential character access with column tracking.
    #[must_use]
    fn new(raw: &'a str) -> Self {
        Self {
            inner: raw.chars().peekable(),
            col: 1,
        }
    }

    /// Consumes and returns the next character with its 1-based column.
    fn next(&mut self) -> Option<(usize, char)> {
        let c = self.inner.next()?;
        let col = self.col;
        self.col = self.col.saturating_add(1);
        Some((col, c))
    }

    /// The next character without consuming it.
    #[must_use]
    fn peek_char(&mut self) -> Option<char> {
        self.inner.peek().copied()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::missing_asserts_for_indexing,
        reason = "test code: asserting exact line/column positions is the whole point of S-004 AC1"
    )]

    use super::*;
    use crate::serializer::{serialize, serialize_document};
    use diagram_model::objects::Object;

    /// Asserts the parse fails at exactly `line`/`column`; returns the error for message checks.
    fn assert_error(input: &str, line: usize, column: usize) -> ParseError {
        let err = parse(input).expect_err("must fail");
        assert_eq!(err.line, line, "line should be {line}, full error: {err}");
        assert_eq!(
            err.column, column,
            "column should be {column}, full error: {err}"
        );
        err
    }

    #[test]
    fn parses_empty_diagram() {
        let diagram = parse("ad 1.0\n").expect("valid");
        assert!(diagram.is_empty());
        assert_eq!(diagram.scale(), 1);
    }

    #[test]
    fn parses_full_example_with_scale_and_all_statement_kinds() {
        let input = "ad 1.0\nscale 2\nshape box \"Server A\" as s1 at 1 1 w 8 h 4\ntext as t1 \"hi\" at 3 2\nconnect as c1 from s1.e to s1.w dashed\n";
        let diagram = parse(input).expect("valid");
        assert_eq!(diagram.scale(), 2);
        assert_eq!(diagram.len(), 3);
        match &diagram.objects()[0] {
            Object::Shape(shape) => {
                assert_eq!(shape.type_id, "box");
                assert_eq!(shape.label, "Server A");
                assert_eq!(shape.pos, CellPos::new(1, 1));
                assert_eq!(shape.size.width, 8);
                assert_eq!(shape.size.height, 4);
            }
            other => panic!("first object must be a shape, got {other:?}"),
        }
        match &diagram.objects()[2] {
            Object::Connector(connector) => assert_eq!(connector.style, ConnectorStyle::Dashed),
            other => panic!("third object must be a connector, got {other:?}"),
        }
    }

    #[test]
    fn missing_header_is_an_error_at_line_1() {
        let err = assert_error("shape box \"x\" as s1 at 0 0 w 1 h 1\n", 1, 1);
        assert!(err.message.contains("header"), "message: {}", err.message);
    }

    #[test]
    fn empty_input_is_a_missing_header_error() {
        let err = assert_error("", 1, 1);
        assert!(
            err.message.contains("missing header"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn unsupported_major_version_rejects_the_file() {
        let err = assert_error("ad 2.0\nscale 1\n", 1, 4);
        assert!(
            err.message.contains("unsupported major version 2"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn header_with_trailing_tokens_is_an_error() {
        let err = assert_error("ad 1.0 extra\n", 1, 8);
        assert!(
            err.message.contains("unexpected token"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn leading_zero_integer_reports_the_token_column() {
        //                                        1234567890123456
        let err = assert_error("ad 1.0\nscale 01\n", 2, 7);
        assert!(
            err.message.contains("leading zeros"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn signed_integer_is_rejected() {
        // The '-' of `-1` is at column 19.
        let err = assert_error("ad 1.0\ntext as t1 \"x\" at -1 0\n", 2, 19);
        assert!(err.message.contains("sign"), "message: {}", err.message);
    }

    #[test]
    fn unterminated_string_reports_the_opening_quote_column() {
        // The opening quote is at column 12.
        let err = assert_error("ad 1.0\ntext as t1 \"abc at 0 0\n", 2, 12);
        assert!(
            err.message.contains("unterminated string"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn invalid_escape_reports_the_escape_column() {
        //                                        1234567890123456789
        let err = assert_error("ad 1.0\ntext as t1 \"a\\qb\" at 0 0\n", 2, 14);
        assert!(
            err.message.contains("invalid escape"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn truncated_statement_reports_missing_keyword() {
        let err = assert_error("ad 1.0\nshape box \"x\" as s1 at 0 0 w 2\n", 2, 1);
        assert!(
            err.message.contains("truncated"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn unexpected_keyword_reports_found_token_column() {
        // The stray token `a` starts at column 15.
        let err = assert_error("ad 1.0\nshape box \"x\" a s1 at 0 0 w 2 h 1\n", 2, 15);
        assert!(
            err.message.contains("expected 'as'"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn trailing_tokens_after_complete_statement_are_rejected() {
        // `junk` starts at column 23.
        let err = assert_error("ad 1.0\ntext as t1 \"x\" at 0 0 junk\n", 2, 23);
        assert!(
            err.message.contains("unexpected token"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn free_endpoint_with_non_numeric_coordinates_is_rejected() {
        // A bare token without a dot is a free endpoint — its x must be an integer (CR-010).
        let input =
            "ad 1.0\nshape box \"x\" as s1 at 0 0 w 2 h 1\nconnect as c1 from s1 0 to s1.w\n";
        let err = assert_error(input, 3, 1);
        assert!(
            err.message.contains("endpoint") && err.message.contains("x coordinate"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn anchor_ref_with_empty_parts_is_rejected() {
        let input = "ad 1.0\nshape box \"x\" as s1 at 0 0 w 2 h 1\nconnect as c1 from .e to s1.w\n";
        let err = assert_error(input, 3, 1);
        assert!(
            err.message.contains("non-empty"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn anchor_ref_splits_at_last_dot() {
        let input = "ad 1.0\nshape box \"x\" as s.a at 0 0 w 2 h 1\nconnect as c1 from s.a.north-east to s.a.w\n";
        let diagram = parse(input).expect("valid");
        match &diagram.objects()[1] {
            Object::Connector(connector) => {
                assert_eq!(connector.from.object().expect("attached").as_str(), "s.a");
                assert_eq!(connector.from.anchor(), Some("north-east"));
            }
            other => panic!("expected connector, got {other:?}"),
        }
    }

    /// A free endpoint is two bare integers and survives the model (CR-010).
    #[test]
    fn free_endpoints_parse_to_cell_positions() {
        let input = "ad 1.0\nconnect as c1 from 2 3 to 9 12\n";
        let diagram = parse(input).expect("valid");
        match &diagram.objects()[0] {
            Object::Connector(connector) => {
                assert_eq!(connector.from, ConnectorEndpoint::Free(CellPos::new(2, 3)));
                assert_eq!(connector.to, ConnectorEndpoint::Free(CellPos::new(9, 12)));
            }
            other => panic!("expected connector, got {other:?}"),
        }
    }

    /// Endpoints mix freely per end — attached source, free target (CR-010).
    #[test]
    fn endpoints_mix_attached_and_free() {
        let input =
            "ad 1.0\nshape box \"x\" as s1 at 0 0 w 2 h 1\nconnect as c1 from s1.e to 5 6 dashed\n";
        let diagram = parse(input).expect("valid");
        match &diagram.objects()[1] {
            Object::Connector(connector) => {
                assert_eq!(
                    connector.from,
                    ConnectorEndpoint::Attached(
                        AnchorRef::new(ObjectId::new("s1").expect("id"), "e").expect("anchor")
                    )
                );
                assert_eq!(connector.to, ConnectorEndpoint::Free(CellPos::new(5, 6)));
                assert_eq!(connector.style, ConnectorStyle::Dashed);
            }
            other => panic!("expected connector, got {other:?}"),
        }
    }

    /// A free endpoint missing its y coordinate is a truncated statement.
    #[test]
    fn free_endpoint_missing_y_is_rejected() {
        let err = assert_error("ad 1.0\nconnect as c1 from 2 to s1.w\n", 2, 22);
        assert!(
            err.message.contains("y coordinate"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn duplicate_scale_is_rejected() {
        let err = assert_error("ad 1.0\nscale 2\nscale 3\n", 3, 1);
        assert!(
            err.message.contains("duplicate"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn scale_after_objects_is_rejected() {
        let input = "ad 1.0\ntext as t1 \"x\" at 0 0\nscale 2\n";
        let err = assert_error(input, 3, 1);
        assert!(err.message.contains("precede"), "message: {}", err.message);
    }

    #[test]
    fn scale_zero_is_a_model_error_with_position() {
        let err = assert_error("ad 1.0\nscale 0\n", 2, 1);
        assert!(
            err.message.contains("at least 1"),
            "message: {}",
            err.message
        );
    }

    /// FR15 — the `colors` statement sets presentation colors on the aggregate (CR-009).
    #[test]
    fn colors_statement_sets_roles() {
        let input = "ad 1.1\nscale 2\ncolors background \"#ff0000\" fill \"#ffffff\" lines \"#0a1b2c\"\ntext as t1 \"x\" at 0 0\n";
        let diagram = parse(input).expect("valid");
        assert_eq!(
            diagram.colors().get(ColorRole::Background),
            Some(Rgb::new(255, 0, 0))
        );
        assert_eq!(
            diagram.colors().get(ColorRole::Fill),
            Some(Rgb::new(255, 255, 255))
        );
        assert_eq!(
            diagram.colors().get(ColorRole::LineText),
            Some(Rgb::new(10, 27, 44))
        );
    }

    /// A subset of roles in any order within the statement parses; hex is case-insensitive.
    #[test]
    fn colors_subset_and_order_independent() {
        let input = "ad 1.1\ncolors lines \"#AABBCC\" background \"#000001\"\n";
        let diagram = parse(input).expect("valid");
        assert_eq!(
            diagram.colors().get(ColorRole::LineText),
            Some(Rgb::new(0xaa, 0xbb, 0xcc))
        );
        assert_eq!(
            diagram.colors().get(ColorRole::Background),
            Some(Rgb::new(0, 0, 1))
        );
        assert_eq!(diagram.colors().get(ColorRole::Fill), None);
    }

    /// A v1.0 file without a `colors` statement still parses (backward compatibility).
    #[test]
    fn v1_0_file_without_colors_parses() {
        let diagram = parse("ad 1.0\nscale 1\ntext as t1 \"x\" at 0 0\n").expect("valid");
        assert!(!diagram.colors().has_any());
    }

    #[test]
    fn colors_unknown_role_is_rejected_with_position() {
        let err = assert_error("ad 1.1\ncolors border \"#ff0000\"\n", 2, 8);
        assert!(
            err.message.contains("unknown color role 'border'"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn colors_duplicate_role_in_one_statement_is_rejected() {
        let err = assert_error("ad 1.1\ncolors fill \"#ff0000\" fill \"#00ff00\"\n", 2, 23);
        assert!(
            err.message.contains("duplicate color role 'fill'"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn colors_invalid_hex_is_rejected_with_position() {
        // The quoted token starts at its opening quote (column 13).
        let err = assert_error("ad 1.1\ncolors fill \"#zzz000\"\n", 2, 13);
        assert!(
            err.message.contains("not a valid color"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn colors_unquoted_hex_is_rejected() {
        // A bare `#hex` would be a comment — the unquoted branch fires on a bare word.
        let err = assert_error("ad 1.1\ncolors fill ff0000\n", 2, 13);
        assert!(
            err.message.contains("expected a quoted"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn colors_empty_statement_is_rejected() {
        let err = assert_error("ad 1.1\ncolors\n", 2, 1);
        assert!(err.message.contains("empty"), "message: {}", err.message);
    }

    #[test]
    fn colors_duplicate_statement_is_rejected() {
        let err = assert_error(
            "ad 1.1\ncolors fill \"#ff0000\"\ncolors lines \"#00ff00\"\n",
            3,
            1,
        );
        assert!(
            err.message.contains("duplicate 'colors' statement"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn colors_after_objects_is_rejected() {
        let input = "ad 1.1\ntext as t1 \"x\" at 0 0\ncolors fill \"#ff0000\"\n";
        let err = assert_error(input, 3, 1);
        assert!(err.message.contains("precede"), "message: {}", err.message);
    }

    #[test]
    fn colors_truncated_after_role_is_rejected() {
        let err = assert_error("ad 1.1\ncolors fill\n", 2, 1);
        assert!(
            err.message.contains("truncated"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn duplicate_id_is_a_model_error_with_position() {
        let input = "ad 1.0\ntext as t1 \"x\" at 0 0\ntext as t1 \"y\" at 1 1\n";
        let err = assert_error(input, 3, 1);
        assert!(
            err.message.contains("already exists"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn connector_to_missing_object_is_a_model_error() {
        let input =
            "ad 1.0\nshape box \"x\" as s1 at 0 0 w 2 h 1\nconnect as c1 from s1.e to ghost.w\n";
        let err = assert_error(input, 3, 1);
        assert!(
            err.message.contains("unknown object id: ghost"),
            "message: {}",
            err.message
        );
    }

    /// S-043 AC3 — mutation audit: the zero-length error must point at the connector's
    /// own declaration (`declared` lookup), surviving neither an inverted id comparison
    /// nor a deleted match arm (which would fall back to (1, 1)). Zero length is only
    /// checkable in the model for both-free endpoints (see `reject_zero_length`).
    #[test]
    fn zero_length_connector_reports_its_declaration_position() {
        let input = "ad 1.0\nshape box \"x\" as s1 at 0 0 w 2 h 2\ntext as t1 \"y\" at 1 1\nconnect as c1 from 3 3 to 3 3\n";
        let err = assert_error(input, 4, 1);
        assert!(
            err.message.contains("zero length") || err.message.contains("same cell"),
            "message: {}",
            err.message
        );
    }

    /// S-043 AC3 — mutation audit: the referencing-connector lookup must match on the
    /// referenced id (guard), not just take the first connector in the file.
    #[test]
    fn unknown_object_error_points_at_the_connector_that_references_it() {
        let input = "ad 1.0\nshape box \"x\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.w to s1.e\nconnect as c2 from s1.n to ghost.s\n";
        let err = assert_error(input, 4, 1);
        assert!(
            err.message.contains("unknown object id: ghost"),
            "message: {}",
            err.message
        );
    }

    /// S-043 — forward references are legal: a connector may reference a shape declared
    /// *later* in the file (file order = z-order; the model allows this state via CR-010's
    /// standalone endpoints). The parsed diagram preserves file order.
    #[test]
    fn connector_may_reference_a_shape_declared_later() {
        let input = "ad 1.1\nscale 2\nconnect as c1 from s2.n to 1 1\ntext as t1 \"x\" at 0 0\nshape box \"\" as s2 at 5 5 w 2 h 2\n";
        let diagram = parse(input).expect("forward reference is legal");
        let objects = diagram.objects();
        assert_eq!(objects.len(), 3);
        // File order (= z-order) is preserved: connector first, then text, then shape.
        assert!(matches!(&objects[0], Object::Connector(c) if c.id.as_str() == "c1"));
        assert!(matches!(&objects[2], Object::Shape(s) if s.id.as_str() == "s2"));
    }

    /// S-043 — a forward reference to non-shape (text) is rejected at the *referencing*
    /// connector statement, not at the referenced object.
    #[test]
    fn forward_reference_to_text_is_rejected_at_the_connector() {
        let input = "ad 1.0\nconnect as c1 from t1.e to 1 1\ntext as t1 \"x\" at 0 0\n";
        let err = assert_error(input, 2, 1);
        assert!(
            err.message.contains("expected a shape"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn connector_to_text_object_is_rejected() {
        let input = "ad 1.0\ntext as t1 \"x\" at 0 0\nconnect as c1 from t1.e to t1.w\n";
        let err = assert_error(input, 3, 1);
        assert!(
            err.message.contains("expected a shape"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn empty_text_content_is_rejected() {
        let err = assert_error("ad 1.0\ntext as t1 \"\" at 0 0\n", 2, 1);
        assert!(err.message.contains("empty"), "message: {}", err.message);
    }

    #[test]
    fn unknown_statement_in_same_minor_file_is_a_hard_error() {
        let err = assert_error("ad 1.0\nfoo bar baz\n", 2, 1);
        assert!(
            err.message.contains("unknown statement"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn higher_minor_parses_and_preserves_unknowns_verbatim() {
        let input = "ad 1.3\nscale 2\ntext as t1 \"x\" at 0 0\nfoo bar   baz # trailing comment\n";
        let doc = parse_document(input).expect("forward-compat file parses");
        assert_eq!(doc.diagram.scale(), 2);
        assert_eq!(doc.diagram.len(), 1);
        assert_eq!(doc.unknown_count(), 1);
        // Re-emitted byte-for-byte at its original position (QS7 across minor upgrades).
        let out = serialize_document(&doc);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[3], "foo bar   baz # trailing comment");
    }

    /// S-043 AC3 — mutation audit: `unknown_count` must reflect the actual number of
    /// preserved statements (the existing test only ever asserts `== 1`, which a
    /// constant-1 mutant survives).
    #[test]
    fn unknown_count_counts_multiple_preserved_statements() {
        let input = "ad 1.3\nfoo one\nbar two three\n";
        let doc = parse_document(input).expect("forward-compat file parses");
        assert_eq!(doc.unknown_count(), 2);
    }

    /// S-043 AC3 — mutation audit: the forward-compat gate is strictly `> MINOR` (1.1);
    /// an unknown statement in a *current*-minor (1.1) file must stay a hard error.
    /// The existing test uses 1.0, which both `>` and `>=` reject.
    #[test]
    fn unknown_statement_in_current_minor_file_is_a_hard_error() {
        let err = assert_error("ad 1.1\nfoo bar baz\n", 2, 1);
        assert!(
            err.message.contains("unknown statement"),
            "message: {}",
            err.message
        );
    }

    /// S-043 AC4 — B-009: a lone trailing `\r` (not part of a CRLF pair) is whitespace,
    /// not word content. Before the fix, `colors\r` tokenized as one unknown word and
    /// was silently preserved in higher-minor files; the re-emitted clean line then
    /// failed to re-parse ("'colors' statement is empty").
    #[test]
    fn lone_cr_at_eof_does_not_mask_a_keyword() {
        let err = assert_error("ad 1.3\n colors\r", 2, 2);
        assert!(err.message.contains("colors"), "message: {}", err.message);
    }

    /// S-043 AC4 — B-009 companion: an *interior* `\r` in a preserved unknown line is
    /// content and must round-trip verbatim (it is not adjacent to the line break, so
    /// no CRLF stripping happens on re-read).
    #[test]
    fn unknown_line_with_interior_cr_round_trips() {
        let input = "ad 1.3\nfoo\rbar baz\n";
        let doc = parse_document(input).expect("valid");
        let out = serialize_document(&doc);
        assert!(out.contains("foo\rbar baz"), "serialized: {out:?}");
        assert_eq!(parse_document(&out).expect("re-saves losslessly"), doc);
    }

    #[test]
    fn same_minor_file_with_only_known_statements_parses() {
        let diagram = parse("ad 1.0\ntext as t1 \"x\" at 0 0\n").expect("valid");
        assert_eq!(diagram.len(), 1);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let input = "ad 1.0\n# a comment\ntext as t1 \"x\" at 0 0\n\n   # indented comment\n";
        let diagram = parse(input).expect("valid");
        assert_eq!(diagram.len(), 1);
    }

    #[test]
    fn comment_inside_string_is_content() {
        let diagram = parse("ad 1.0\ntext as t1 \"a # not a comment\" at 0 0\n").expect("valid");
        match &diagram.objects()[0] {
            Object::Text(text) => assert_eq!(text.text, "a # not a comment"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn crlf_line_endings_are_tolerated() {
        let diagram = parse("ad 1.0\r\ntext as t1 \"x\" at 0 0\r\n").expect("valid");
        assert_eq!(diagram.len(), 1);
    }

    #[test]
    fn extra_whitespace_between_tokens_is_tolerated() {
        let diagram = parse("ad 1.0\nscale\t2\ntext   as    t1 \"x\"   at  0  0\n").expect("valid");
        assert_eq!(diagram.scale(), 2);
        assert_eq!(diagram.len(), 1);
    }

    #[test]
    fn escape_sequences_round_trip() {
        let input =
            "ad 1.0\ntext as t1 \"a \\\"quote\\\", a \\\\ backslash, and a newline:\\nb\" at 0 0\n";
        let diagram = parse(input).expect("valid");
        match &diagram.objects()[0] {
            Object::Text(text) => {
                assert_eq!(text.text, "a \"quote\", a \\ backslash, and a newline:\nb")
            }
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn serialize_is_canonical_and_stable() {
        let input = "ad 1.0\nscale\t2\ntext   as    t1 \"x\"   at  0  0\n";
        let diagram = parse(input).expect("valid");
        assert_eq!(
            serialize(&diagram),
            "ad 1.1\nscale 2\ntext as t1 \"x\" at 0 0\n"
        );
    }

    /// AC2 (S-037): shape statements accept zero or more `region <name> "<content>"`
    /// clauses after the size — content uses the standard escapes and may be empty,
    /// clauses appear in any order, and region-free files parse unchanged.

    #[test]
    fn shape_region_clauses_parse() {
        let input = "ad 1.1\nscale 2\nshape box \"Srv\" as s1 at 1 1 w 8 h 4 region title \"Server A\" region body \"\\nempty ok: \\\"q\\\"\"\n";
        let diagram = parse(input).expect("valid");
        match &diagram.objects()[0] {
            Object::Shape(shape) => {
                assert_eq!(shape.region_content("title"), Some("Server A"));
                assert_eq!(shape.region_content("body"), Some("\nempty ok: \"q\""));
                assert_eq!(shape.region_content("nope"), None);
            }
            other => panic!("expected shape, got {other:?}"),
        }
    }

    /// AC2 (S-037): region clauses round-trip — serialized by name after the size,
    /// and a shape without regions serializes byte-identically to the old form.
    #[test]
    fn shape_region_clauses_round_trip() {
        let input = "ad 1.1\nscale 2\nshape box \"Srv\" as s1 at 1 1 w 8 h 4 region zeta \"z\" region alpha \"a\"\n";
        let diagram = parse(input).expect("valid");
        assert_eq!(
            serialize(&diagram),
            "ad 1.1\nscale 2\nshape box \"Srv\" as s1 at 1 1 w 8 h 4 region alpha \"a\" region zeta \"z\"\n"
        );

        let plain =
            parse("ad 1.0\nscale 2\nshape box \"x\" as s1 at 0 0 w 2 h 1\n").expect("valid");
        assert_eq!(
            serialize(&plain),
            "ad 1.1\nscale 2\nshape box \"x\" as s1 at 0 0 w 2 h 1\n"
        );
    }

    /// CR-021: the compartmented class carries all three region clauses — parsed into
    /// the instance's per-region content and re-emitted canonically (sorted by name),
    /// so a saved diagram restores every compartment verbatim.
    #[test]
    fn class_shape_carries_its_three_region_clauses_through_round_trip() {
        // The `\n` inside the quoted attribute content is the file-level escape for a
        // line break (C6); written as \\n in this Rust literal.
        let input = "ad 1.1\nscale 2\nshape class \"\" as c1 at 0 0 w 13 h 10 region methods \"move()\" region name \"Animal\" region attributes \"+ legs: int\\n- age: int\"\n";
        let diagram = parse(input).expect("valid class statement");
        match &diagram.objects()[0] {
            Object::Shape(shape) => {
                assert_eq!(shape.type_id, "class");
                assert_eq!(shape.region_content("name"), Some("Animal"));
                assert_eq!(
                    shape.region_content("attributes"),
                    Some("+ legs: int\n- age: int")
                );
                assert_eq!(shape.region_content("methods"), Some("move()"));
            }
            other => panic!("expected shape, got {other:?}"),
        }
        let out = serialize(&diagram);
        assert_eq!(
            out,
            "ad 1.1\nscale 2\nshape class \"\" as c1 at 0 0 w 13 h 10 region attributes \"+ legs: int\\n- age: int\" region methods \"move()\" region name \"Animal\"\n"
        );
        assert_eq!(parse(&out).expect("re-parse"), diagram);
    }

    /// AC2 (S-037): a region name declared twice in one shape statement is an error.
    #[test]
    fn shape_duplicate_region_clause_is_an_error() {
        let err = assert_error(
            "ad 1.1\nscale 2\nshape box \"x\" as s1 at 0 0 w 2 h 1 region r \"a\" region r \"b\"\n",
            3,
            1,
        );
        assert!(err.to_string().contains("more than once"), "{err}");
    }

    #[test]
    fn unknown_statements_preserve_original_position_relative_to_objects() {
        let input = "ad 1.3\ntext as t1 \"x\" at 0 0\nfoo one\ntext as t2 \"y\" at 1 1\nfoo two\n";
        let doc = parse_document(input).expect("valid");
        let out = serialize_document(&doc);
        assert_eq!(
            out,
            // The source version (1.3) is re-emitted: it keeps the preserved `foo`
            // statements legal on re-parse (QS7 across minor-version upgrades).
            "ad 1.3\nscale 1\ntext as t1 \"x\" at 0 0\nfoo one\ntext as t2 \"y\" at 1 1\nfoo two\n"
        );
    }

    #[test]
    fn error_columns_count_multibyte_chars_as_one() {
        // 'î' is two bytes but one column: the stray token `a` starts at column 17.
        let input = "ad 1.0\nshape boîte \"x\" a s1 at 0 0 w 1 h 1\n";
        let err = assert_error(input, 2, 17);
        assert!(
            err.message.contains("expected 'as'"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn blank_and_comment_lines_between_statements_are_ignored() {
        let input = "ad 1.0\n# a comment\n\nscale 2\n\ntext as t1 \"x\" at 0 0 # inline\n";
        let diagram = parse(input).expect("valid");
        assert_eq!(
            serialize(&diagram),
            "ad 1.1\nscale 2\ntext as t1 \"x\" at 0 0\n"
        );
    }

    #[test]
    fn backslash_at_end_of_line_in_string_is_an_error() {
        // The backslash must be the last character of its physical line.
        let err = assert_error("ad 1.0\nshape box \"abc\\\n", 2, 15);
        assert!(
            err.message.contains("unterminated escape"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn lenient_three_token_header_version_is_accepted() {
        let diagram = parse("ad 1 . 0\nscale 2\n").expect("valid");
        assert_eq!(serialize(&diagram), "ad 1.1\nscale 2\n");
    }

    #[test]
    fn lenient_header_without_dot_is_an_error() {
        let err = assert_error("ad 1 x 0\n", 1, 6);
        assert!(
            err.message.contains("expected '.'"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn header_missing_minor_version_is_an_error() {
        let _ = assert_error("ad 1 .\n", 1, 1);
    }

    #[test]
    fn non_decimal_integer_is_an_error() {
        let err = assert_error("ad 1.0\nscale abc\n", 2, 7);
        assert!(
            err.message.contains("not a decimal integer"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn quoted_empty_string_as_integer_is_an_error() {
        let _ = assert_error("ad 1.0\nscale \"\"\n", 2, 7);
    }

    #[test]
    fn connect_with_trailing_token_is_an_error() {
        let input = "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nshape box \"b\" as s2 at 5 0 w 2 h 2\nconnect as c1 from s1.a to s2.b junk\n";
        let err = assert_error(input, 4, 33);
        assert!(
            err.message.contains("only 'via …' may follow here"),
            "message: {}",
            err.message
        );
    }

    /// `via` waypoints parse into the connector's bend list (CR-007) — with or without `dashed`.
    #[test]
    fn via_waypoints_parse_into_bends() {
        let input = "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nshape box \"b\" as s2 at 5 0 w 2 h 2\nconnect as c1 from s1.e to s2.w dashed via 3 1, 4 4\n";
        let diagram = parse(input).expect("valid");
        let Object::Connector(connector) = &diagram.objects()[2] else {
            panic!("expected connector")
        };
        assert_eq!(connector.style, ConnectorStyle::Dashed);
        assert_eq!(
            connector.bends,
            vec![CellPos::new(3, 1), CellPos::new(4, 4)]
        );
    }

    /// S-043 AC3 — mutation audit: a comma is its own token even when glued to the
    /// following number (`via 3 4,5 6`); deleting the dedicated `,` tokenizer arm would
    /// let `read_word` swallow `,5` and reject this legal input.
    #[test]
    fn via_list_accepts_comma_without_trailing_space() {
        let input = "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nshape box \"b\" as s2 at 5 0 w 2 h 2\nconnect as c1 from s1.e to s2.w via 3 1,4 4\n";
        let diagram = parse(input).expect("valid");
        let Object::Connector(connector) = &diagram.objects()[2] else {
            panic!("expected connector")
        };
        assert_eq!(
            connector.bends,
            vec![CellPos::new(3, 1), CellPos::new(4, 4)]
        );
    }

    /// S-043 AC3 — mutation audit: a missing separator in a `via` list must be reported
    /// as an *unexpected token* (not silently consumed by an over-eager comma guard).
    #[test]
    fn via_list_missing_separator_is_an_unexpected_token() {
        let input = "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w via 3 4 5\n";
        let err = parse(input).expect_err("must fail");
        assert_eq!(err.line, 3);
        assert!(
            err.message.contains("unexpected token"),
            "message: {}",
            err.message
        );
    }

    /// A `via` list needs complete `<x> <y>` pairs — truncation and garbage are line/column errors.
    #[test]
    fn via_list_errors_are_positioned() {
        // Truncated pair (missing y) — error at the statement end.
        let err = assert_error(
            "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w via 3\n",
            3,
            1,
        );
        assert!(
            err.message.contains("via y coordinate"),
            "message: {}",
            err.message
        );

        // Dangling comma separator (missing the next pair).
        let err = assert_error(
            "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w via 3 4,\n",
            3,
            1,
        );
        assert!(
            err.message.contains("via x coordinate"),
            "message: {}",
            err.message
        );

        // Non-integer waypoint — error at the offending token.
        let err = assert_error(
            "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w via x 4\n",
            3,
            37,
        );
        assert!(
            err.message.contains("via x coordinate"),
            "message: {}",
            err.message
        );
    }

    /// `dashed` without `via` still parses (bends stay empty) — no behavior change for v1 files.
    #[test]
    fn dashed_without_via_keeps_empty_bends() {
        let input = "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w dashed\n";
        let diagram = parse(input).expect("valid");
        let Object::Connector(connector) = &diagram.objects()[1] else {
            panic!("expected connector")
        };
        assert_eq!(connector.style, ConnectorStyle::Dashed);
        assert!(connector.bends.is_empty());
    }

    /// A token after the optional clauses that is not `via` is an error naming the clause order.
    #[test]
    fn token_after_dashed_must_be_via() {
        let err = assert_error(
            "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w dashed junk\n",
            3,
            40,
        );
        assert!(
            err.message.contains("only 'via …' may follow here"),
            "message: {}",
            err.message
        );
    }

    /// A `tip <src> <tgt>` clause parses both per-end styles (CR-008).
    #[test]
    fn tip_clause_parses_per_end_styles() {
        let input = "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nshape box \"b\" as s2 at 5 0 w 2 h 2\nconnect as c1 from s1.e to s2.w tip rhombus-open rhombus-filled\n";
        let diagram = parse(input).expect("valid");
        let Object::Connector(connector) = &diagram.objects()[2] else {
            panic!("expected connector")
        };
        assert_eq!(connector.from_tip, TipStyle::EmptyRhombus);
        assert_eq!(connector.to_tip, TipStyle::FilledRhombus);
    }

    /// Without a `tip` clause the CR-008 defaults apply: source bare, target hollow arrow.
    #[test]
    fn tip_defaults_when_clause_absent() {
        let input = "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w dashed via 3 1\n";
        let diagram = parse(input).expect("valid");
        let Object::Connector(connector) = &diagram.objects()[1] else {
            panic!("expected connector")
        };
        assert_eq!(connector.from_tip, TipStyle::Bare);
        assert_eq!(connector.to_tip, TipStyle::HollowArrow);
    }

    /// Unknown tip keywords and truncated clauses are line/column errors (CR-008).
    #[test]
    fn tip_clause_errors_are_positioned() {
        // Unknown keyword — error at the offending token.
        let err = assert_error(
            "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w tip triangle arrow\n",
            3,
            37,
        );
        assert!(
            err.message.contains("unknown tip style 'triangle'"),
            "message: {}",
            err.message
        );

        // Truncated clause (missing the target style) — error at the statement end.
        let err = assert_error(
            "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nconnect as c1 from s1.e to s1.w tip arrow\n",
            3,
            1,
        );
        assert!(
            err.message.contains("tip style"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn truncated_shape_statement_reports_missing_coordinate() {
        let _ = assert_error("ad 1.0\nshape box \"a\" as s1 at 0\n", 2, 1);
    }

    #[test]
    fn truncated_connect_statement_reports_missing_anchor() {
        let input = "ad 1.0\nshape box \"a\" as s1 at 0 0 w 2 h 2\nshape box \"b\" as s2 at 5 0 w 2 h 2\nconnect as c1 from s1.a to\n";
        let _ = assert_error(input, 4, 1);
    }

    #[test]
    fn quoted_type_id_is_an_error() {
        let err = assert_error("ad 1.0\nshape \"box\" \"a\" as s1 at 0 0 w 2 h 2\n", 2, 7);
        assert!(
            err.message.contains("bare word"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn shape_missing_label_is_truncated() {
        let _ = assert_error("ad 1.0\nshape box\n", 2, 1);
    }

    #[test]
    fn shape_with_bare_label_is_an_error() {
        let err = assert_error("ad 1.0\nshape box at 0 0 w 2 h 2 as s1\n", 2, 11);
        assert!(
            err.message.contains("quoted string"),
            "message: {}",
            err.message
        );
    }
}
