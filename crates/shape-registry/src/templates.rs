//! Parser: `.adc` collection file text → [`CollectionFile`] / [`Collection`], per the
//! normative grammar in [shape-template](../../../doc/arc42/12-concepts/shape-template.md)
//! and [collection](../../../doc/arc42/12-concepts/collection.md).
//!
//! Lenient on input, strict on output: runs of whitespace, CRLF line endings, `#`
//! comments (on statement lines), and blank lines between blocks are tolerated; every
//! known statement must be well-formed. Unknown statements are preserved verbatim only
//! when the file's minor version is higher than this reader's (forward compatibility, QS7).

use std::collections::BTreeMap;

use crate::error::{AnchorSetError, ParseError, RegionSetError};
use crate::geometry::{CellPos, Region};
use crate::types::{
    AnchorSet, Collection, EdgeRule, GeometryKind, RegionSet, ResizeRules, ShapeType, TemplateGrid,
};

/// The major format version this reader implements.
const MAJOR: u32 = 1;
/// The minor version this reader implements; files with a higher minor are read in forward-compat mode.
/// 1.1 adds `resize` statements (C10, S-024) and `region` statements (FR19, S-037).
const MINOR: u32 = 1;

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
            // C10 (S-024): resize rules exist from v1.1 up; in older files the
            // statement falls through to the unknown-statement arm below.
            "resize" if version.1 >= MINOR => {
                let Some(builder) = open.as_mut() else {
                    return Err(ParseError::new(
                        line_no,
                        first.column,
                        "resize statement outside a template block — resize rules belong to exactly one template",
                    ));
                };
                parse_resize(&tokens, line_no, builder)?;
            }
            // FR19 (S-037): text regions exist from v1.1 up; in older files the
            // statement falls through to the unknown-statement arm below.
            "region" if version.1 >= MINOR => {
                let Some(builder) = open.as_mut() else {
                    return Err(ParseError::new(
                        line_no,
                        first.column,
                        "region statement outside a template block — regions belong to exactly one template",
                    ));
                };
                parse_region(&tokens, line_no, builder)?;
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
                            "unknown statement '{other}' — version {}.{} files may only contain known statements",
                            version.0, version.1
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

/// A template edge named by a `resize` statement (C10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Edge {
    /// The top edge (row 0's run between its corner cells).
    Top,
    /// The bottom edge (last row's run).
    Bottom,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
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
    /// Text regions declared in the anchor phase (FR19); each rect was bounds-checked
    /// against the declared grid as it arrived.
    regions: RegionSet,
    /// Resize rules declared in the anchor phase: edge → draft (base + optional parity
    /// overrides). All four base rules are required — checked on finalize (C10).
    resize: BTreeMap<Edge, EdgeDraft>,
    /// 1-based line of the last `resize` statement in this block (error context for
    /// finalize-time checks such as the all-four-edges rule).
    resize_last_line: usize,
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
    let resize_rules =
        build_resize_rules(&builder.name, builder.resize_last_line, &builder.resize)?;
    types.push(ShapeType {
        type_id: format!("{collection_name}/{}", builder.name),
        name: builder.name,
        // Custom templates are always stamped opaquely at natural size (ADR 007);
        // resizing re-rasterizes the pattern per its rules (C10, S-024).
        geometry: GeometryKind::Pattern,
        glyph_pattern,
        anchor_set: builder.anchors,
        regions: builder.regions,
        resize_rules,
    });
    slots.push(Slot::Template(types.len() - 1, builder.unknowns));
    Ok(())
}

/// Builds the template's rule set from its declared edges: no statements keep the
/// template fixed-size; a partial set is an error (rules are all-or-nothing, C10).
fn build_resize_rules(
    name: &str,
    last_resize_line: usize,
    edges: &BTreeMap<Edge, EdgeDraft>,
) -> Result<Option<ResizeRules>, ParseError> {
    if edges.is_empty() {
        return Ok(None);
    }
    // A rule set is complete only when every edge declares its BASE pair; parity
    // overrides are optional per edge (C10).
    let missing: Vec<&str> = [
        (Edge::Top, "top"),
        (Edge::Bottom, "bottom"),
        (Edge::Left, "left"),
        (Edge::Right, "right"),
    ]
    .into_iter()
    .filter(|(edge, _)| !edges.get(edge).is_some_and(|d| d.base.is_some()))
    .map(|(_, label)| label)
    .collect();
    if !missing.is_empty() {
        return Err(ParseError::new(
            last_resize_line,
            1,
            format!(
                "template '{name}' declares resize rules for some edges only — all four base rules are required (missing: {})",
                missing.join(", ")
            ),
        ));
    }
    let rule = |draft: &EdgeDraft| EdgeRule {
        insert: draft.base.expect("checked above").0,
        remove: draft.base.expect("checked above").1,
        even: draft.even,
        odd: draft.odd,
    };
    Ok(Some(ResizeRules {
        top: rule(&edges[&Edge::Top]),
        bottom: rule(&edges[&Edge::Bottom]),
        left: rule(&edges[&Edge::Left]),
        right: rule(&edges[&Edge::Right]),
    }))
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
        regions: RegionSet::default(),
        resize: BTreeMap::new(),
        resize_last_line: 0,
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

/// Parses `region <name> at <x> <y> w <w> h <h>` (keyword consumed) into the open
/// template's region set. The rect must fit inside the declared grid with extents ≥ 1;
/// names are single tokens, unique per template (FR19).
fn parse_region(
    tokens: &[Token],
    line_no: usize,
    builder: &mut TemplateBuilder,
) -> Result<(), ParseError> {
    let what = "region statement — expected 'region <name> at <x> <y> w <w> h <h>'";
    let name_tok = need(tokens, 1, line_no, what)?;
    let name = name_tok.as_str().to_owned();
    let at_tok = need(tokens, 2, line_no, what)?;
    if at_tok.as_str() != "at" {
        return Err(ParseError::new(
            line_no,
            at_tok.column,
            format!(
                "expected 'at' in the region statement, found '{}'",
                at_tok.as_str()
            ),
        ));
    }
    let x = parse_int(need(tokens, 3, line_no, what)?, line_no)?;
    let y = parse_int(need(tokens, 4, line_no, what)?, line_no)?;
    let w_kw = need(tokens, 5, line_no, what)?;
    if w_kw.as_str() != "w" {
        return Err(ParseError::new(
            line_no,
            w_kw.column,
            format!(
                "expected 'w' after the region origin, found '{}'",
                w_kw.as_str()
            ),
        ));
    }
    let width = parse_int(need(tokens, 6, line_no, what)?, line_no)?;
    let h_kw = need(tokens, 7, line_no, what)?;
    if h_kw.as_str() != "h" {
        return Err(ParseError::new(
            line_no,
            h_kw.column,
            format!(
                "expected 'h' after the region width, found '{}'",
                h_kw.as_str()
            ),
        ));
    }
    let height = parse_int(need(tokens, 8, line_no, what)?, line_no)?;
    expect_no_leftover(&tokens[9..], line_no, "the region statement")?;
    if width == 0 || height == 0 {
        return Err(ParseError::new(
            line_no,
            name_tok.column,
            "region extents must each be at least 1",
        ));
    }
    let rect = Region::new(x, y, width, height);
    if !rect.fits(builder.width, builder.height) {
        return Err(ParseError::new(
            line_no,
            name_tok.column,
            format!(
                "region '{name}' at ({x}, {y}) with extent {width} x {height} does not fit the {} x {} template grid",
                builder.width, builder.height
            ),
        ));
    }
    if let Err(RegionSetError::DuplicateName(_)) = builder.regions.add(&name, rect) {
        return Err(ParseError::new(
            line_no,
            name_tok.column,
            format!(
                "region '{name}' is already declared in template '{}'",
                builder.name
            ),
        ));
    }
    Ok(())
}

/// Parses `resize <edge> [even | odd] insert "<char>" remove "<char>"` (keyword
/// consumed) into the open template's rule set. A statement without a parity keyword
/// declares the edge's base insert/remove pair; `even`/`odd` statements declare
/// overrides used when the target extent has that parity (C10). At most one
/// statement per (edge, parity); all four base rules are required to enable resizing.
///
/// Per-edge resize rule draft while parsing: the base pair plus at most one parity
/// override each. `resize` statements are all-or-nothing per edge kind — duplicates
/// are rejected as they arrive (C10).
#[derive(Debug, Default)]
struct EdgeDraft {
    /// The base insert/remove pair (required for a complete rule set).
    base: Option<(char, char)>,
    /// Insert/remove pair used when the target extent is even.
    even: Option<crate::types::ParityChars>,
    /// Insert/remove pair used when the target extent is odd.
    odd: Option<crate::types::ParityChars>,
}

fn parse_resize(
    tokens: &[Token],
    line_no: usize,
    builder: &mut TemplateBuilder,
) -> Result<(), ParseError> {
    let what = "resize statement — expected 'resize <edge> [even | odd] insert \"<char>\" remove \"<char>\"'";
    let edge_tok = need(tokens, 1, line_no, what)?;
    let (edge, label) = match edge_tok.as_str() {
        "top" => (Edge::Top, "top"),
        "bottom" => (Edge::Bottom, "bottom"),
        "left" => (Edge::Left, "left"),
        "right" => (Edge::Right, "right"),
        other => {
            return Err(ParseError::new(
                line_no,
                edge_tok.column,
                format!(
                    "unknown resize edge '{other}' — expected 'top', 'bottom', 'left' or 'right'"
                ),
            ));
        }
    };
    // Optional parity keyword between the edge and the `insert` keyword.
    let mut rest = &tokens[2..];
    let parity: Option<bool> = match rest.first().map(|t| t.as_str()) {
        Some("even") => {
            rest = &rest[1..];
            Some(true)
        }
        Some("odd") => {
            rest = &rest[1..];
            Some(false)
        }
        _ => None,
    };
    let ins_kw = need(rest, 0, line_no, what)?;
    if ins_kw.as_str() != "insert" {
        return Err(ParseError::new(
            line_no,
            ins_kw.column,
            format!(
                "expected the 'insert' keyword after the resize edge — found '{}'",
                ins_kw.as_str()
            ),
        ));
    }
    let insert = parse_char_token(need(rest, 1, line_no, what)?, line_no)?;
    let rem_kw = need(rest, 2, line_no, what)?;
    if rem_kw.as_str() != "remove" {
        return Err(ParseError::new(
            line_no,
            rem_kw.column,
            format!(
                "expected the 'remove' keyword after the insert character — found '{}'",
                rem_kw.as_str()
            ),
        ));
    }
    let remove = parse_char_token(need(rest, 3, line_no, what)?, line_no)?;
    if rest.len() > 4 {
        return Err(ParseError::new(
            line_no,
            rest[4].column,
            format!(
                "unexpected token '{}' after the remove character",
                rest[4].as_str()
            ),
        ));
    }

    let draft = builder.resize.entry(edge).or_default();
    let pair = (insert, remove);
    match parity {
        None => {
            if draft.base.is_some() {
                return Err(ParseError::new(
                    line_no,
                    edge_tok.column,
                    format!(
                        "the base resize rule for the '{label}' edge of template '{}' is already declared",
                        builder.name
                    ),
                ));
            }
            draft.base = Some(pair);
        }
        Some(true) => {
            if draft.even.is_some() {
                return Err(ParseError::new(
                    line_no,
                    edge_tok.column,
                    format!(
                        "the even parity override for the '{label}' edge of template '{}' is already declared",
                        builder.name
                    ),
                ));
            }
            draft.even = Some(crate::types::ParityChars { insert, remove });
        }
        Some(false) => {
            if draft.odd.is_some() {
                return Err(ParseError::new(
                    line_no,
                    edge_tok.column,
                    format!(
                        "the odd parity override for the '{label}' edge of template '{}' is already declared",
                        builder.name
                    ),
                ));
            }
            draft.odd = Some(crate::types::ParityChars { insert, remove });
        }
    }
    builder.resize_last_line = line_no;
    Ok(())
}

/// Parses a quoted single-character token `"<c>"`: exactly one printable
/// non-whitespace ASCII character between double quotes.
fn parse_char_token(token: &Token, line_no: usize) -> Result<char, ParseError> {
    let chars: Vec<char> = token.as_str().chars().collect();
    if chars.len() != 3 || chars[0] != '"' || chars[2] != '"' {
        return Err(ParseError::new(
            line_no,
            token.column,
            format!(
                "expected a quoted single character like '\"-\"', found '{}'",
                token.as_str()
            ),
        ));
    }
    let c = chars[1];
    if !matches!(c as u32, 0x21..=0x7e) {
        return Err(ParseError::new(
            line_no,
            token.column + 1,
            format!(
                "resize characters must be printable non-whitespace ASCII — found U+{:04X}",
                c as u32
            ),
        ));
    }
    Ok(c)
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
        assert_eq!(gear.resize_rules, None); // no resize statements → fixed-size (C10)
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

    /// Forward-compat: a higher-minor file (beyond this reader's 1.1) preserves
    /// unknown statements verbatim, in position.
    #[test]
    fn forward_compat_preserves_unknown_statements() {
        let text = "adc 1.2\nmeta author \"me\"\ntemplate t 2 x 2\nab\ncd\nhint prefer solid\n";
        let file = parse_ok("c", text);
        assert_eq!(file.unknown_count(), 2);
        // The template itself still parses normally.
        assert_eq!(file.collection.types.len(), 1);
    }

    /// A statement unknown to this reader in a same-version (1.1) file is rejected —
    /// preservation applies only to *higher* minors.
    #[test]
    fn rejects_unknown_statements_in_same_version_file() {
        let err = assert_err(
            "c",
            "adc 1.1\nmeta author \"me\"\ntemplate t 2 x 2\nab\ncd\n",
        );
        assert_eq!(err.line, 2);
        assert!(err.message.contains("unknown statement 'meta'"), "{err}");
        assert!(err.message.contains("version 1.1 files"), "{err}");
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

    /// AC2 (S-024): a v1.1 file with complete resize rules parses into the rule set,
    /// including parity overrides and distinct insert/remove pairs; other statements
    /// are unaffected, and statement order among anchors is free.
    #[test]
    fn parses_resize_rules_with_parity_overrides() {
        let text = "adc 1.1\ntemplate box 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"-\" remove \"-\"\nanchor n at 2 0\nresize top even insert \"=\" remove \"=\"\nresize bottom insert \"-\" remove \"=\"\nresize left insert \"|\" remove \"|\"\nresize right insert \"|\" remove \"!\"\nresize right odd insert \"!\" remove \"!\"\n";
        let file = parse_ok("c", text);
        let box_ = &file.collection.types[0];
        let rules = box_.resize_rules.expect("complete rule set parses");
        assert_eq!(rules.top.insert, '-');
        assert_eq!(rules.top.remove, '-');
        assert_eq!(
            rules.top.even,
            Some(crate::types::ParityChars {
                insert: '=',
                remove: '='
            })
        );
        assert_eq!(rules.top.odd, None);
        // Insert and remove may differ per edge.
        assert_eq!(rules.bottom.insert, '-');
        assert_eq!(rules.bottom.remove, '=');
        assert_eq!(rules.left.insert, '|');
        assert_eq!(rules.right.insert, '|');
        assert_eq!(rules.right.remove, '!');
        assert_eq!(
            rules.right.odd,
            Some(crate::types::ParityChars {
                insert: '!',
                remove: '!'
            })
        );
        // Anchors and the grid parse alongside the rules.
        assert_eq!(box_.anchor_set.get("n"), Some(CellPos::new(2, 0)));
        assert_eq!(box_.glyph_pattern.row(1), Some("|  |".to_owned()));
    }

    /// A template without resize statements stays fixed-size in a v1.1 file.
    #[test]
    fn v1_1_file_without_resize_statements_stays_fixed_size() {
        let file = parse_ok("c", "adc 1.1\ntemplate t 2 x 2\nab\ncd\n");
        assert_eq!(file.collection.types[0].resize_rules, None);
    }

    /// AC1 (S-037): a v1.1 file with region statements parses them into the template's
    /// region set — names unique, rects as declared, in any order among anchors.
    #[test]
    fn parses_region_statements() {
        let text = "adc 1.1\ntemplate box 6 x 4\n+----+\n|    |\n|    |\n+----+\nanchor n at 3 0\nregion title at 1 1 w 4 h 1\nregion body at 1 2 w 4 h 1\n";
        let file = parse_ok("c", text);
        let box_ = &file.collection.types[0];
        assert_eq!(box_.regions.len(), 2);
        assert_eq!(
            box_.regions.get("title"),
            Some(crate::geometry::Region::new(1, 1, 4, 1))
        );
        assert_eq!(
            box_.regions.get("body"),
            Some(crate::geometry::Region::new(1, 2, 4, 1))
        );
        // Anchors and the grid parse alongside the regions.
        assert_eq!(box_.anchor_set.get("n"), Some(CellPos::new(3, 0)));
    }

    /// A template without region statements has an empty region set in a v1.1 file.
    #[test]
    fn v1_1_file_without_region_statements_has_no_regions() {
        let file = parse_ok("c", "adc 1.1\ntemplate t 2 x 2\nab\ncd\n");
        assert!(file.collection.types[0].regions.is_empty());
    }

    /// AC1 (S-037): region statement errors — out-of-bounds rects, zero extents,
    /// duplicate names, missing keywords, trailing junk, statements outside blocks,
    /// and 1.0 files (the statement is unknown there).
    #[test]
    fn region_statement_errors_are_typed_and_located() {
        // Out of bounds: the rect must fit inside the declared grid.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nregion r at 2 1 w 4 h 1\n",
        );
        assert!(err.message.contains("does not fit"), "{err}");

        // Zero extents are rejected.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nregion r at 1 1 w 0 h 1\n",
        );
        assert!(err.message.contains("at least 1"), "{err}");

        // Duplicate region names within one template.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nregion r at 1 1 w 2 h 1\nregion r at 1 2 w 2 h 1\n",
        );
        assert!(err.message.contains("already declared"), "{err}");

        // Missing the 'at' keyword.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nregion r 1 1 w 2 h 1\n",
        );
        assert!(err.message.contains("expected 'at'"), "{err}");

        // Trailing junk after the last number.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nregion r at 1 1 w 2 h 1 extra\n",
        );
        assert!(err.message.contains("unexpected token"), "{err}");

        // A region statement before any template block.
        let err = assert_err(
            "c",
            "adc 1.1\nregion r at 1 1 w 2 h 1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\n",
        );
        assert!(err.message.contains("outside a template block"), "{err}");

        // 1.0 files: the statement is unknown there (forward-compat rule).
        let err = assert_err(
            "c",
            "adc 1.0\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nregion r at 1 1 w 2 h 1\n",
        );
        assert!(err.message.contains("'region'"), "{err}");
    }

    /// AC2 (S-024): resize statement errors — partial rule sets, duplicate base and
    /// parity declarations, missing keywords, bad character tokens, trailing junk,
    /// unknown edges, statements outside blocks, and 1.0 files.
    #[test]
    fn resize_statement_errors_are_typed_and_located() {
        // Partial rule set: all four BASE rules are required once any is declared.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"-\" remove \"-\"\nresize left insert \"|\" remove \"|\"\n",
        );
        assert!(
            err.message.contains("all four base rules are required"),
            "{err}"
        );
        assert!(err.message.contains("missing: bottom, right"), "{err}");

        // A parity override alone does not count as the edge's base rule.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top even insert \"=\" remove \"=\"\nresize bottom insert \"-\" remove \"-\"\nresize left insert \"|\" remove \"|\"\nresize right insert \"|\" remove \"|\"\n",
        );
        assert!(err.message.contains("missing: top"), "{err}");

        // Duplicate base rule for an edge.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"-\" remove \"-\"\nresize top insert \"=\" remove \"=\"\n",
        );
        assert_eq!(err.line, 7);
        assert!(err.message.contains("base resize rule"), "{err}");
        assert!(err.message.contains("already declared"), "{err}");

        // Duplicate parity override for an edge.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"-\" remove \"-\"\nresize top even insert \"=\" remove \"=\"\nresize top even insert \"~\" remove \"~\"\n",
        );
        assert_eq!(err.line, 8);
        assert!(err.message.contains("even parity override"), "{err}");

        // Missing the `insert` keyword.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top \"-\" remove \"-\"\n",
        );
        assert_eq!(err.line, 6);
        assert!(
            err.message.contains("expected the 'insert' keyword"),
            "{err}"
        );

        // Missing the `remove` keyword.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"-\" junk\n",
        );
        assert!(
            err.message.contains("expected the 'remove' keyword"),
            "{err}"
        );

        // Unquoted character token.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert - remove \"-\"\n",
        );
        assert!(err.message.contains("quoted single character"), "{err}");

        // Quoted multi-character token.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"--\" remove \"-\"\n",
        );
        assert!(err.message.contains("quoted single character"), "{err}");

        // Whitespace characters are not declarable (space fills are auto-derived):
        // a quoted space is two quote tokens, which fails the character check.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert \" \" remove \"-\"\n",
        );
        assert!(err.message.contains("quoted single character"), "{err}");

        // Garbage after the remove character.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"-\" remove \"-\" junk\n",
        );
        assert!(err.message.contains("unexpected token 'junk'"), "{err}");

        // Unknown edge keyword.
        let err = assert_err(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize north insert \"-\" remove \"-\"\n",
        );
        assert!(err.message.contains("unknown resize edge 'north'"), "{err}");

        // Resize statement outside a template block.
        let err = assert_err(
            "c",
            "adc 1.1\nresize top insert \"-\" remove \"-\"\ntemplate t 1 x 1\n.\n",
        );
        assert_eq!(err.line, 2);
        assert!(err.message.contains("outside a template block"), "{err}");

        // A 1.0 file cannot contain resize statements (unknown-statement rejection).
        let err = assert_err(
            "c",
            "adc 1.0\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"-\" remove \"-\"\n",
        );
        assert_eq!(err.line, 6);
        assert!(err.message.contains("unknown statement 'resize'"), "{err}");
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
