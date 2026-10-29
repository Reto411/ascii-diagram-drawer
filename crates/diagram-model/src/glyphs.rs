//! Classification of the svgbob ASCII output alphabet (ADR 001 v2, concept C1).
//!
//! These predicates are shared by every consumer that must tell structural glyphs from
//! literal content — the render pipeline (quoting per C1 F4, scale expansion per FR8) and
//! the GUI (hit-testing of free-text objects, S-018). One definition here keeps them in
//! lockstep.

/// Whether `c` is a structural line glyph of the svgbob alphabet: it connects to its
/// neighbors and fills its whole scale block when the grid is expanded (FR8). The quote
/// character is content, not structure.
pub const fn is_line_glyph(c: char) -> bool {
    matches!(
        c,
        '+' | '-' | '|' | '.' | ',' | ':' | ';' | '/' | '\\' | '_' | '`' | '\'' | '(' | ')'
    )
}

/// Whether `c` is an arrowhead glyph produced by connector routing (ADR 013): solid
/// `< > ^ v`, stamped as content at the target end of a route.
pub const fn is_arrowhead(c: char) -> bool {
    matches!(c, '<' | '>' | '^' | 'v')
}

/// Whether `c` belongs to svgbob's ASCII input alphabet (ADR 001 v2): the structural line
/// glyphs, double quotes, and the arrow vocabulary `< > ^ v V`. (The pipeline only ever
/// emits the lowercase arrows as connector heads; uppercase `V` appears in user text,
/// where it must still count for quote-escaping per C1 F4.)
pub const fn is_svgbob_glyph(c: char) -> bool {
    c == '"' || c == 'V' || is_line_glyph(c) || is_arrowhead(c)
}
