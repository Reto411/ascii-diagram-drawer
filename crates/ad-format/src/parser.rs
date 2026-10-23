//! Parser: `.ad` text → [`Document`] / [`Diagram`], per the normative grammar in
//! [diagram-file-format.md](../../../doc/arc42/12-concepts/diagram-file-format.md).
//!
//! Lenient in, strict out: whitespace runs, CRLF endings and `#` comments are tolerated;
//! every known statement must be well-formed. Unknown statements are preserved verbatim
//! only when the file's minor version is higher than this reader's (forward compatibility,
//! QS7 across minor upgrades).

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::invariants::ModelError;
use diagram_model::objects::{
    ConnectorInstance, ConnectorStyle, ObjectId, ShapeInstance, TextInstance,
};

use crate::error::ParseError;

/// The major format version this reader implements.
const MAJOR: u32 = 1;
/// The minor version this reader implements; files with a higher minor are read in forward-compat mode.
const MINOR: u32 = 0;

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
    let mut diagram = Diagram::new();
    let mut slots: Vec<Slot> = Vec::new();
    let mut header_seen = false;
    let mut version = (MAJOR, MINOR);
    let mut scale_seen = false;

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
            "scale" => parse_scale(&tokens, line_no, &mut scale_seen, &mut diagram)?,
            "shape" => {
                let mut cursor = Cursor::new(&tokens);
                cursor.next(); // consume the matched "shape" keyword
                let shape = parse_shape(&mut cursor, line_no)?;
                slots.push(Slot::Object(diagram.len()));
                diagram
                    .place_shape(shape)
                    .map_err(|e| model_error(e, line_no, first.column))?;
            }
            "text" => {
                let mut cursor = Cursor::new(&tokens);
                cursor.next(); // consume the matched "text" keyword
                let text = parse_text(&mut cursor, line_no)?;
                slots.push(Slot::Object(diagram.len()));
                diagram
                    .place_text(text)
                    .map_err(|e| model_error(e, line_no, first.column))?;
            }
            "connect" => {
                let mut cursor = Cursor::new(&tokens);
                cursor.next(); // consume the matched "connect" keyword
                let connector = parse_connect(&mut cursor, line_no)?;
                slots.push(Slot::Object(diagram.len()));
                diagram
                    .connect(connector)
                    .map_err(|e| model_error(e, line_no, first.column))?;
            }
            other => {
                if version.1 > MINOR {
                    slots.push(Slot::Unknown(raw_line.to_owned()));
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
    Ok(Document {
        diagram,
        slots,
        version,
    })
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
            ' ' | '\t' => {}
            '#' => return Ok(tokens),
            '"' => tokens.push(read_string(&mut stream, line_no, col)?),
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
        if matches!(next, ' ' | '\t' | '"' | '#') {
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
    if !diagram.is_empty() {
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
    cursor.expect_end("shape statement", line_no)?;

    let size = CellSize::new(width, height).map_err(|e| model_error(e, line_no, 1))?;
    ShapeInstance::new(
        ObjectId::new(&id).map_err(|e| model_error(e, line_no, 1))?,
        &type_id,
        &label,
        CellPos::new(x, y),
        size,
    )
    .map_err(|e| model_error(e, line_no, 1))
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

/// Parses `connect as <id> from <ref> to <ref> [dashed]` (keyword consumed).
fn parse_connect(cursor: &mut Cursor, line_no: usize) -> Result<ConnectorInstance, ParseError> {
    cursor.expect_keyword("as", line_no)?;
    let id = cursor.expect_word("an id", line_no)?;
    cursor.expect_keyword("from", line_no)?;
    let from = parse_anchor_ref(
        cursor.expect_word("the 'from' anchor-ref", line_no)?,
        line_no,
    )?;
    cursor.expect_keyword("to", line_no)?;
    let to = parse_anchor_ref(cursor.expect_word("the 'to' anchor-ref", line_no)?, line_no)?;
    let style = match cursor.peek() {
        Some(dashed) if dashed.as_str() == "dashed" => {
            cursor.next();
            ConnectorStyle::Dashed
        }
        Some(extra) => {
            return Err(ParseError::new(
                line_no,
                extra.column,
                format!(
                    "unexpected token '{}' — only 'dashed' may follow the endpoints",
                    extra.as_str()
                ),
            ));
        }
        None => ConnectorStyle::Solid,
    };
    let object_id = ObjectId::new(&id).map_err(|e| model_error(e, line_no, 1))?;
    Ok(ConnectorInstance::new(object_id, from, to, style))
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
    fn anchor_ref_without_dot_is_rejected() {
        let input = "ad 1.0\nshape box \"x\" as s1 at 0 0 w 2 h 1\nconnect as c1 from s1 to s1\n";
        let err = assert_error(input, 3, 1);
        assert!(
            err.message.contains("no dot found"),
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
                assert_eq!(connector.from.object().as_str(), "s.a");
                assert_eq!(connector.from.anchor(), "north-east");
            }
            other => panic!("expected connector, got {other:?}"),
        }
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
            "ad 1.0\nscale 2\ntext as t1 \"x\" at 0 0\n"
        );
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
            "ad 1.0\nscale 2\ntext as t1 \"x\" at 0 0\n"
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
        assert_eq!(serialize(&diagram), "ad 1.0\nscale 2\n");
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
            err.message.contains("only 'dashed'"),
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
