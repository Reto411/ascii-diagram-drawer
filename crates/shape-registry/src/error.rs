//! Typed errors for the shape catalog. Every fallible operation returns one of these —
//! no panics on malformed input, and `.adc` parse errors carry 1-based line/column
//! positions in the same style as `ad-format`.

use std::fmt;

/// A structural problem with a glyph grid ([`TemplateGrid`](crate::types::TemplateGrid)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    /// No rows (or only empty rows) — a template needs at least one cell.
    Empty,
    /// Rows have different lengths — a grid is rectangular.
    RaggedRows,
    /// A cell holds a character outside printable ASCII U+0020–U+007E.
    InvalidChar {
        /// 0-based row index of the offending cell.
        row: usize,
        /// 0-based column index of the offending cell.
        column: usize,
        /// The offending character.
        char: char,
    },
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "a glyph grid needs at least one non-empty row"),
            Self::RaggedRows => {
                write!(
                    f,
                    "glyph grid rows have different lengths — a grid is rectangular"
                )
            }
            Self::InvalidChar { row, column, char } => write!(
                f,
                "grid cell ({column}, {row}) holds U+{:04X} — only printable ASCII (U+0020–U+007E) is allowed",
                *char as u32
            ),
        }
    }
}

impl std::error::Error for GridError {}

/// An anchor-set invariant violation ([`AnchorSet`](crate::types::AnchorSet)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorSetError {
    /// The anchor name is already declared in this set.
    DuplicateName(String),
}

impl fmt::Display for AnchorSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateName(name) => write!(f, "anchor '{name}' is already declared"),
        }
    }
}

impl std::error::Error for AnchorSetError {}

/// A type-id could not be resolved against the registry (C8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    /// A bare token that matches no built-in shape.
    UnknownBuiltin {
        /// The type-id as it appeared in the diagram.
        id: String,
    },
    /// The `collection` part of a qualified id names no loaded collection.
    UnknownCollection {
        /// The collection name from the qualified id.
        collection: String,
        /// The template name from the qualified id.
        name: String,
    },
    /// The collection exists but declares no template with this name.
    UnknownTemplate {
        /// The collection name from the qualified id.
        collection: String,
        /// The template name from the qualified id.
        name: String,
    },
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBuiltin { id } => write!(f, "unknown shape type '{id}' — not a built-in"),
            Self::UnknownCollection { collection, name } => write!(
                f,
                "collection '{collection}' is not loaded (looking up '{collection}/{name}')"
            ),
            Self::UnknownTemplate { collection, name } => write!(
                f,
                "template '{name}' is not declared in collection '{collection}'"
            ),
        }
    }
}

impl std::error::Error for LookupError {}

/// A `.adc` collection file is malformed. Carries the 1-based position of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based column where the problem starts.
    pub column: usize,
    /// 1-based line number where the problem occurs.
    pub line: usize,
    /// What is wrong, in human-readable form.
    pub message: String,
}

impl ParseError {
    /// Creates a parse error at `line` (1-based) and `column` (1-based).
    #[must_use]
    pub fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Display strings are user-facing (they surface in GUI notices) — pin them.
    #[test]
    fn display_strings_are_stable() {
        assert_eq!(
            GridError::Empty.to_string(),
            "a glyph grid needs at least one non-empty row"
        );
        assert_eq!(
            GridError::RaggedRows.to_string(),
            "glyph grid rows have different lengths — a grid is rectangular"
        );
        assert_eq!(
            GridError::InvalidChar {
                row: 1,
                column: 2,
                char: 'é'
            }
            .to_string(),
            "grid cell (2, 1) holds U+00E9 — only printable ASCII (U+0020–U+007E) is allowed"
        );
        assert_eq!(
            AnchorSetError::DuplicateName("n".to_owned()).to_string(),
            "anchor 'n' is already declared"
        );
        assert_eq!(
            LookupError::UnknownBuiltin {
                id: "widget".into()
            }
            .to_string(),
            "unknown shape type 'widget' — not a built-in"
        );
        assert_eq!(
            LookupError::UnknownCollection {
                collection: "col".into(),
                name: "t".into(),
            }
            .to_string(),
            "collection 'col' is not loaded (looking up 'col/t')"
        );
        assert_eq!(
            LookupError::UnknownTemplate {
                collection: "col".into(),
                name: "t".into(),
            }
            .to_string(),
            "template 't' is not declared in collection 'col'"
        );
        let err = ParseError::new(3, 7, "expected template");
        assert_eq!(err.to_string(), "line 3, column 7: expected template");
    }
}
