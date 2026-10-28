//! Parser: `.adc` collection file text → [`CollectionFile`] / [`Collection`], per the
//! normative grammar in [shape-template](../../../doc/arc42/12-concepts/shape-template.md)
//! and [collection](../../../doc/arc42/12-concepts/collection.md).
//!
//! Lenient on input, strict on output: runs of whitespace, CRLF line endings, `#`
//! comments (on statement lines), and blank lines between blocks are tolerated; every
//! known statement must be well-formed. Unknown statements are preserved verbatim only
//! when the file's minor version is higher than this reader's (forward compatibility, QS7).

use crate::error::{AnchorSetError, ParseError};
use crate::geometry::CellPos;
use crate::types::{AnchorSet, Collection, GeometryKind, ShapeType, TemplateGrid};

/// The major format version this reader implements.
const MAJOR: u32 = 1;
/// The minor version this reader implements; files with a higher minor are read in forward-compat mode.
const MINOR: u32 = 0;

/// A parsed `.adc` file: the collection content plus unknown statements preserved
/// verbatim, so re-saving after a format upgrade loses nothing (QS7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionFile {
    /// The parsed collection (identity + templates in declaration order).
    pub collection: Collection,
    /// Statement slots at their original file positions; `Template` indices point into
    /// the template list as of parse time and go stale if the collection is mutated after loading.
    pub(crate) slots: Vec<Slot>,
    /// The source file's header version. Re-emitted on save so preserved unknown
    /// statements remain legal (forward-compat round trip, QS7).
    pub(crate) version: (u32, u32),
}

/// One statement slot of the original file, preserving its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Slot {
    /// A template block; `0`-index into [`Collection::types`] as of parse time, with the
    /// unknown statements declared inside its anchor phase (after the grid rows), in file order.
    Template(usize, Vec<String>),
    /// An unknown top-level statement preserved verbatim (forward-compat mode only).
    Unknown(String),
}

impl CollectionFile {
    /// The number of preserved unknown statements (top-level and inside template blocks).
    #[must_use]
    pub fn unknown_count(&self) -> usize {
        self.slots
            .iter()
            .map(|slot| match slot {
                Slot::Template(_, unknowns) => unknowns.len(),
                Slot::Unknown(_) => 1,
            })
            .sum()
    }
}

/// Parses `.adc` collection file text into a [`CollectionFile`].
///
/// `name` is the collection identity (filename stem, case-sensitive) — the file itself
/// carries no name statement.
///
/// # Errors
/// [`ParseError`] with 1-based line/column on any malformed input: missing or bad header,
/// unsupported major version, truncated or ill-formed template declaration, a grid row
/// longer than the declared width or holding a non-printable-ASCII character, an anchor
/// statement outside a block or out of bounds, duplicate names, or a file that ends
/// before all declared grid rows are present. Never panics on input.
pub fn parse_collection(name: &str, text: &str) -> Result<CollectionFile, ParseError> {
    let mut types: Vec<ShapeType> = Vec::new();
    let mut slots: Vec<Slot> = Vec::new();
    let mut version = (MAJOR, MINOR);
    let mut header_seen = false;
    let mut open: Option<TemplateBuilder> = None;

    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index.saturating_add(1);
        // CRLF tolerated: strip the trailing CR before any interpretation.
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

        // Grid phase: exactly `height` physical lines are consumed as grid rows, whatever they contain.
        if let Some(builder) = &mut open
            && builder.rows.len() < builder.height as usize
        {
            parse_grid_row(line, line_no, builder)?;
            continue;
        }

        // Statement phase: top level or the anchor phase of the open template.
        let tokens = tokenize(line);
        if tokens.is_empty() {
            continue; // blank line or comment-only line: decoration, no model information
        }
        let Some(first) = tokens.first() else {
            unreachable!("a non-empty token list has a first token");
        };

        if !header_seen {
            version = parse_header(&tokens, line_no)?;
            header_seen = true;
            continue;
        }

        match first.as_str() {
            "template" => {
                // The open block's anchor phase ends where the next declaration begins.
                if let Some(previous) = open.take() {
                    finalize(name, &mut types, &mut slots, previous)?;
                }
                let builder = parse_template_decl(&tokens, line_no, &types)?;
                open = Some(builder);
            }
            "anchor" => {
                let Some(builder) = open.as_mut() else {
                    return Err(ParseError::new(
                        line_no,
                        first.column,
                        "anchor statement outside a template block — anchors belong to exactly one template",
                    ));
                };
                parse_anchor(&tokens, line_no, builder)?;
            }
            other => {
                if version.1 > MINOR {
                    // Forward-compat: preserve verbatim in its original position — inside the open
                    // block (after its grid rows) or at top level.
                    match open.as_mut() {
                        Some(builder) => builder.unknowns.push(line.to_owned()),
                        None => slots.push(Slot::Unknown(line.to_owned())),
                    }
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

    // End of file: close the last block; a grid left incomplete is an error.
    if let Some(builder) = open {
        if builder.rows.len() < builder.height as usize {
            return Err(ParseError::new(
                text.lines().count().max(1),
                1,
                format!(
                    "template '{}' expected {} grid rows after its declaration, found {}",
                    builder.name,
                    builder.height,
                    builder.rows.len()
                ),
            ));
        }
        finalize(name, &mut types, &mut slots, builder)?;
    }

    if !header_seen {
        return Err(ParseError::new(
            1,
            1,
            "missing header — the first statement must be 'adc <major>.<minor>'",
        ));
    }

    Ok(CollectionFile {
        collection: Collection {
            name: name.to_owned(),
            types,
        },
        slots,
        version,
    })
}

/// A template block being read: declaration, grid rows consumed so far, anchor statements.
struct TemplateBuilder {
    /// The declared template name (unique within the collection).
    name: String,
    /// Declared width in cells (≥ 1).
    width: u32,
    /// Declared height in cells (≥ 1); exactly this many physical lines are grid rows.
    height: u32,
    /// Grid rows consumed verbatim (each ≤ `width` chars; right-padded on finalize).
    rows: Vec<String>,
    /// Anchor statements declared in the block's anchor phase.
    anchors: AnchorSet,
    /// Unknown statements in the anchor phase, preserved verbatim in file order.
    unknowns: Vec<String>,
}

/// Closes a completed template block: pads grid rows to width, builds the shape type,
/// and records its slot (with preserved unknowns) at its file position.
fn finalize(
    collection_name: &str,
    types: &mut Vec<ShapeType>,
    slots: &mut Vec<Slot>,
    builder: TemplateBuilder,
) -> Result<(), ParseError> {
    let padded: Vec<String> = builder
        .rows
        .iter()
        .map(|row| format!("{row:width$}", width = builder.width as usize))
        .collect();
    let rows: Vec<&str> = padded.iter().map(String::as_str).collect();
    // Rows were validated at parse time (length, printable ASCII); `from_rows` re-checks.
    let glyph_pattern =
        TemplateGrid::from_rows(&rows).expect("grid rows were validated at parse time");
    types.push(ShapeType {
        type_id: format!("{collection_name}/{}", builder.name),
        name: builder.name,
        // Custom templates are always stamped opaquely at natural size (ADR 007).
        geometry: GeometryKind::Pattern,
        glyph_pattern,
        anchor_set: builder.anchors,
        resize_rules: None, // v1: all custom objects are fixed-size (ADR 007 v3)
    });
    slots.push(Slot::Template(types.len() - 1, builder.unknowns));
    Ok(())
}

/// Consumes one physical line as a grid row of the open template: at most `width`
/// printable-ASCII characters (shorter rows are right-padded with spaces on finalize).
fn parse_grid_row(
    line: &str,
    line_no: usize,
    builder: &mut TemplateBuilder,
) -> Result<(), ParseError> {
    for (column, c) in line.chars().enumerate() {
        if column >= builder.width as usize {
            return Err(ParseError::new(
                line_no,
                column + 1,
                format!(
                    "grid row is {} cells long — template '{}' declares width {}",
                    line.chars().count(),
                    builder.name,
                    builder.width
                ),
            ));
        }
        if !matches!(c as u32, 0x20..=0x7e) {
            return Err(ParseError::new(
                line_no,
                column + 1,
                format!(
                    "grid rows hold printable ASCII (U+0020–U+007E) only — found U+{:04X}",
                    c as u32
                ),
            ));
        }
    }
    builder.rows.push(line.to_owned());
    Ok(())
}

/// Parses `template <name> <width> x <height>` (keyword consumed).
fn parse_template_decl(
    tokens: &[Token],
    line_no: usize,
    existing: &[ShapeType],
) -> Result<TemplateBuilder, ParseError> {
    let what = "template declaration — expected 'template <name> <width> x <height>'";
    let name_tok = need(tokens, 1, line_no, what)?;
    let name = name_tok.as_str().to_owned();
    if existing.iter().any(|t| t.name == name) {
        return Err(ParseError::new(
            line_no,
            name_tok.column,
            format!("template '{name}' is already declared in this collection"),
        ));
    }
    let width = parse_int(need(tokens, 2, line_no, what)?, line_no)?;
    let x_tok = need(tokens, 3, line_no, what)?;
    if x_tok.as_str() != "x" {
        return Err(ParseError::new(
            line_no,
            x_tok.column,
            format!(
                "expected 'x' between the template width and height, found '{}'",
                x_tok.as_str()
            ),
        ));
    }
    let height = parse_int(need(tokens, 4, line_no, what)?, line_no)?;
    expect_no_leftover(&tokens[5..], line_no, "the template declaration")?;
    if width == 0 || height == 0 {
        return Err(ParseError::new(
            line_no,
            name_tok.column,
            "template dimensions must each be at least 1",
        ));
    }
    Ok(TemplateBuilder {
        name,
        width,
        height,
        rows: Vec::new(),
        anchors: AnchorSet::default(),
        unknowns: Vec::new(),
    })
}

/// Parses `anchor <name> at <x> <y>` (keyword consumed) into the open template's anchor set.
fn parse_anchor(
    tokens: &[Token],
    line_no: usize,
    builder: &mut TemplateBuilder,
) -> Result<(), ParseError> {
    let what = "anchor statement — expected 'anchor <name> at <x> <y>'";
    let name_tok = need(tokens, 1, line_no, what)?;
    let name = name_tok.as_str().to_owned();
    let at_tok = need(tokens, 2, line_no, what)?;
    if at_tok.as_str() != "at" {
        return Err(ParseError::new(
            line_no,
            at_tok.column,
            format!(
                "expected 'at' in the anchor statement, found '{}'",
                at_tok.as_str()
            ),
        ));
    }
    let x = parse_int(need(tokens, 3, line_no, what)?, line_no)?;
    let y = parse_int(need(tokens, 4, line_no, what)?, line_no)?;
    expect_no_leftover(&tokens[5..], line_no, "the anchor statement")?;
    if !(x < builder.width && y < builder.height) {
        return Err(ParseError::new(
            line_no,
            name_tok.column,
            format!(
                "anchor '{name}' at ({x}, {y}) is outside the {} x {} template grid",
                builder.width, builder.height
            ),
        ));
    }
    if let Err(AnchorSetError::DuplicateName(_)) = builder.anchors.add(&name, CellPos::new(x, y)) {
        return Err(ParseError::new(
            line_no,
            name_tok.column,
            format!(
                "anchor '{name}' is already declared in template '{}'",
                builder.name
            ),
        ));
    }
    Ok(())
}

/// Parses the header statement `adc <major>.<minor>` and returns its `(major, minor)` version.
fn parse_header(tokens: &[Token], line_no: usize) -> Result<(u32, u32), ParseError> {
    let (kind, after_kind) = tokens.split_first().ok_or_else(|| {
        ParseError::new(
            line_no,
            1,
            "missing header — the first statement must be 'adc <major>.<minor>'",
        )
    })?;
    if kind.as_str() != "adc" {
        return Err(ParseError::new(
            line_no,
            kind.column,
            format!("expected header keyword 'adc', found '{}'", kind.as_str()),
        ));
    }
    let (version_tok, after_version) = split(
        after_kind,
        line_no,
        "header — expected 'adc <major>.<minor>'",
    )?;
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
    let (dot, after_dot) = split(rest, line_no, "header — expected 'adc <major>.<minor>'")?;
    if dot.as_str() != "." {
        return Err(ParseError::new(
            line_no,
            dot.column,
            format!("expected '.' in version number, found '{}'", dot.as_str()),
        ));
    }
    let (minor_tok, leftover) = split(
        after_dot,
        line_no,
        "header — expected 'adc <major>.<minor>'",
    )?;
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
fn split<'a>(
    rest: &'a [Token],
    line_no: usize,
    what: &str,
) -> Result<(&'a Token, &'a [Token]), ParseError> {
    rest.split_first()
        .ok_or_else(|| ParseError::new(line_no, 1, format!("truncated {what}")))
}

/// Fetches the token at `index`, erroring when the statement is truncated.
fn need<'a>(
    tokens: &'a [Token],
    index: usize,
    line_no: usize,
    what: &str,
) -> Result<&'a Token, ParseError> {
    tokens
        .get(index)
        .ok_or_else(|| ParseError::new(line_no, 1, format!("truncated {what}")))
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

/// A lexical token with its 1-based starting column.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    /// 1-based character column where the token starts.
    column: usize,
    /// The token text (non-empty runs of non-space/tab characters).
    text: String,
}

impl Token {
    /// The token text.
    #[must_use]
    fn as_str(&self) -> &str {
        &self.text
    }

    /// Builds a bare-word token.
    #[must_use]
    fn word(text: String, column: usize) -> Self {
        Self { column, text }
    }
}

/// Splits a physical line into tokens; `#` starts a comment to the end of the line.
fn tokenize(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut start = 0usize; // character ordinal where the current token began
    for (ordinal, c) in line.chars().enumerate() {
        if c == '#' {
            break; // comment to end of line
        }
        if matches!(c, ' ' | '\t') {
            if !current.is_empty() {
                tokens.push(Token::word(current.clone(), start + 1));
                current.clear();
            }
        } else {
            if current.is_empty() {
                start = ordinal;
            }
            current.push(c);
        }
    }
    if !current.is_empty() {
        tokens.push(Token::word(current, start + 1));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::{CollectionFile, parse_collection};
    use crate::error::ParseError;
    use crate::geometry::CellPos;

    fn parse_ok(name: &str, text: &str) -> CollectionFile {
        parse_collection(name, text).unwrap_or_else(|e| panic!("expected '{name}' to parse: {e}"))
    }

    fn assert_err(name: &str, text: &str) -> ParseError {
        match parse_collection(name, text) {
            Ok(_) => panic!("expected '{name}' to be rejected"),
            Err(e) => e,
        }
    }

    /// A minimal valid collection: header plus one anchor-less template.
    #[test]
    fn parses_minimal_collection() {
        let file = parse_ok("mycol", "adc 1.0\ntemplate gear 4 x 2\nab\ncd\n");
        assert_eq!(file.collection.name, "mycol");
        assert_eq!(file.collection.types.len(), 1);
        let gear = &file.collection.types[0];
        assert_eq!(gear.type_id, "mycol/gear");
        assert_eq!(gear.glyph_pattern.width(), 4);
        assert_eq!(gear.glyph_pattern.height(), 2);
        assert_eq!(gear.glyph_pattern.row(0), Some("ab  ".to_owned()));
        assert_eq!(gear.glyph_pattern.row(1), Some("cd  ".to_owned()));
        assert!(gear.anchor_set.is_empty());
        assert_eq!(gear.resize_rules, None); // AC4: v1 template shapes have no resize rules
    }

    /// Full file: two templates with anchors, blank lines between blocks, comments.
    #[test]
    fn parses_two_templates_with_anchors_and_decoration() {
        let file = parse_ok(
            "col",
            "adc 1.0\n\ntemplate box 4 x 3\n+--+\n|  |\n+--+\nanchor n at 2 0\nanchor s at 2 2\n# a comment between blocks\n\ntemplate dot 1 x 1\n.\n",
        );
        assert_eq!(file.collection.types.len(), 2);
        let rect = &file.collection.types[0];
        assert_eq!(rect.anchor_set.get("n"), Some(CellPos::new(2, 0)));
        assert_eq!(rect.anchor_set.get("s"), Some(CellPos::new(2, 2)));
        assert_eq!(rect.glyph_pattern.row(1), Some("|  |".to_owned()));
        let dot = &file.collection.types[1];
        assert_eq!(dot.type_id, "col/dot");
        assert_eq!(dot.size().width, 1);
    }

    /// Short grid rows are right-padded with spaces to the declared width.
    #[test]
    fn short_grid_rows_are_padded_to_width() {
        let file = parse_ok("c", "adc 1.0\ntemplate t 5 x 2\nab\nabcd\n");
        let t = &file.collection.types[0];
        assert_eq!(t.glyph_pattern.row(0), Some("ab   ".to_owned()));
        assert_eq!(t.glyph_pattern.row(1), Some("abcd ".to_owned()));
    }

    /// CRLF line endings and tabs as statement separators are tolerated.
    #[test]
    fn tolerates_crlf_and_tab_separators() {
        let file = parse_ok(
            "c",
            "adc 1.0\r\ntemplate\tt\t3 x 2\r\nab\r\ncd\r\nanchor n at 1 0\r\n",
        );
        assert_eq!(file.collection.types.len(), 1);
        assert_eq!(
            file.collection.types[0].anchor_set.get("n"),
            Some(CellPos::new(1, 0))
        );
    }

    /// The lenient three-token header form `adc 1 . 0` parses (same rule as `.ad`).
    #[test]
    fn tolerates_lenient_header_form() {
        let file = parse_ok("c", "adc 1 . 0\ntemplate t 1 x 1\n.\n");
        assert_eq!(file.collection.types.len(), 1);
    }

    /// Forward-compat: a higher-minor file preserves unknown statements verbatim, in position.
    #[test]
    fn forward_compat_preserves_unknown_statements() {
        let text = "adc 1.1\nmeta author \"me\"\ntemplate t 2 x 2\nab\ncd\nhint prefer solid\n";
        let file = parse_ok("c", text);
        assert_eq!(file.unknown_count(), 2);
        // The template itself still parses normally.
        assert_eq!(file.collection.types.len(), 1);
    }

    /// A version 1.0 file with an unknown statement is rejected, not preserved.
    #[test]
    fn rejects_unknown_statements_in_v1_0() {
        let err = assert_err(
            "c",
            "adc 1.0\nmeta author \"me\"\ntemplate t 2 x 2\nab\ncd\n",
        );
        assert_eq!(err.line, 2);
        assert!(err.message.contains("unknown statement 'meta'"), "{err}");
    }

    /// AC2: invalid files produce location-bearing errors and never panic — the required four-plus cases.
    #[test]
    fn invalid_files_produce_location_errors() {
        // 1. Missing header (first statement is a template).
        let err = assert_err("c", "template t 1 x 1\n.\n");
        assert_eq!((err.line, err.column), (1, 1));
        assert!(
            err.message.contains("expected header keyword 'adc'"),
            "{err}"
        );

        // 2. Unsupported major version.
        let err = assert_err("c", "adc 2.0\ntemplate t 1 x 1\n.\n");
        assert_eq!(err.line, 1);
        assert!(err.message.contains("unsupported major version 2"), "{err}");

        // 3. Non-integer version part.
        let err = assert_err("c", "adc one\ntemplate t 1 x 1\n.\n");
        assert_eq!(err.line, 1);
        assert!(err.message.contains("not a decimal integer"), "{err}");

        // 4. Truncated template declaration.
        let err = assert_err("c", "adc 1.0\ntemplate gear 4 x\nab\ncd\nef\ngh\n");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("truncated"), "{err}");

        // 5. Grid row longer than the declared width.
        let err = assert_err("c", "adc 1.0\ntemplate t 3 x 2\nabcd\ncd\n");
        assert_eq!((err.line, err.column), (3, 4));
        assert!(err.message.contains("grid row is 4 cells long"), "{err}");

        // 6. Grid row with a non-printable character (tab).
        let err = assert_err("c", "adc 1.0\ntemplate t 3 x 2\na\tb\ncd\n");
        assert_eq!((err.line, err.column), (3, 2));
        assert!(err.message.contains("U+0009"), "{err}");

        // 7. Anchor statement outside any template block.
        let err = assert_err("c", "adc 1.0\nanchor n at 0 0\ntemplate t 1 x 1\n.\n");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("outside a template block"), "{err}");

        // 8. Anchor outside the template grid.
        let err = assert_err("c", "adc 1.0\ntemplate t 2 x 2\nab\ncd\nanchor n at 5 0\n");
        assert_eq!(err.line, 5);
        assert!(
            err.message.contains("outside the 2 x 2 template grid"),
            "{err}"
        );

        // 9. Duplicate template name in one collection.
        let err = assert_err("c", "adc 1.0\ntemplate t 1 x 1\n.\ntemplate t 1 x 1\n.\n");
        assert_eq!(err.line, 4);
        assert!(err.message.contains("already declared"), "{err}");

        // 10. Duplicate anchor name in one template.
        let err = assert_err(
            "c",
            "adc 1.0\ntemplate t 2 x 2\nab\ncd\nanchor n at 0 0\nanchor n at 1 1\n",
        );
        assert_eq!(err.line, 6);
        assert!(err.message.contains("already declared"), "{err}");

        // 11. File ends before all declared grid rows are present.
        let err = assert_err("c", "adc 1.0\ntemplate t 2 x 3\nab\ncd\n");
        assert_eq!(err.line, 4);
        assert!(err.message.contains("expected 3 grid rows"), "{err}");

        // 12. Wrong separator between width and height.
        let err = assert_err("c", "adc 1.0\ntemplate t 4 y 2\nab\ncd\n");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("expected 'x'"), "{err}");

        // 13. Garbage after a complete declaration.
        let err = assert_err("c", "adc 1.0\ntemplate t 2 x 2 extra\nab\ncd\n");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("unexpected token 'extra'"), "{err}");

        // 14. Empty file: missing header.
        let err = assert_err("c", "");
        assert_eq!((err.line, err.column), (1, 1));
        assert!(err.message.contains("missing header"), "{err}");
    }

    /// Zero dimensions are rejected (a grid needs at least one cell per dimension).
    #[test]
    fn rejects_zero_dimensions() {
        let err = assert_err("c", "adc 1.0\ntemplate t 0 x 2\nab\ncd\n");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("at least 1"), "{err}");
    }

    /// Non-canonical integers (leading zeros) are rejected in declarations.
    #[test]
    fn rejects_non_canonical_integers() {
        let err = assert_err("c", "adc 1.0\ntemplate t 04 x 2\nab\ncd\n");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("leading zeros"), "{err}");
    }

    /// The parsed collection preserves declaration order of its templates.
    #[test]
    fn template_declaration_order_is_preserved() {
        let file = parse_ok("c", "adc 1.0\ntemplate b 1 x 1\n.\ntemplate a 1 x 1\n.\n");
        let names: Vec<&str> = file
            .collection
            .types
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert_eq!(names, vec!["b", "a"]);
    }
}
