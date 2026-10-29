//! Service-level error types for the ADR 009 seam.
//!
//! The seam owns its own error vocabulary: backends map their concrete failure
//! types (parser errors, rasterizer errors) into these, so `gui-app` never sees a
//! context-crate error type — only the seam's language.

/// Failure of a diagram store operation (load/save of `.ad` documents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Platform IO failed — file not found, unreadable, unwritable. The in-memory
    /// store reports an unknown path here too (it has no filesystem to miss on).
    Io {
        /// What went wrong, suitable for display to the user.
        message: String,
    },
    /// The file's content is not a valid `.ad` document.
    Parse {
        /// 1-based physical line where the problem was detected.
        line: usize,
        /// 1-based character column where the problem starts.
        column: usize,
        /// Human-readable description of the problem.
        message: String,
    },
}

impl StoreError {
    /// The 1-based `(line, column)` of a parse failure, if this is one — the GUI's
    /// error dialog jumps to the offending position (S-014).
    #[must_use]
    pub fn line_col(&self) -> Option<(usize, usize)> {
        match self {
            Self::Parse { line, column, .. } => Some((*line, *column)),
            Self::Io { .. } => None,
        }
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message } => write!(f, "file operation failed: {message}"),
            Self::Parse {
                line,
                column,
                message,
            } => write!(f, "parse error at line {line}, column {column}: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// Failure of a grid → media export (SVG/PNG; text export is infallible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// Vector emission failed (the SVG conversion seam — svgbob today, ADR 005).
    Vector {
        /// The upstream error message.
        message: String,
    },
    /// Raster emission failed (SVG parse or PNG encoding inside the rasterizer, ADR 016).
    Raster {
        /// The upstream error message.
        message: String,
    },
    /// No rasterizer backend is available in this build — the GUI offers SVG-only
    /// export without any code change (ADR 009 graceful degradation, risk R3).
    Unsupported,
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vector { message } => write!(f, "SVG export failed: {message}"),
            Self::Raster { message } => write!(f, "PNG export failed: {message}"),
            Self::Unsupported => write!(f, "PNG export is unavailable in this build"),
        }
    }
}

impl std::error::Error for ExportError {}

/// Failure of a shape-catalog operation (template registration, S-017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// Platform IO failed while persisting the collection file — unwritable path,
    /// missing collections directory, ….
    Io {
        /// What went wrong, suitable for display to the user.
        message: String,
    },
    /// The submitted template is not valid catalog data (bad name or grid
    /// characters, out-of-bounds or duplicate anchors, no anchors at all).
    Invalid {
        /// Human-readable description of the problem.
        message: String,
    },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message } => write!(f, "collection file operation failed: {message}"),
            Self::Invalid { message } => write!(f, "invalid template: {message}"),
        }
    }
}

impl std::error::Error for CatalogError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every seam-level error variant formats without panicking — these messages are
    /// what the GUI shows users (S-014/S-015), so they must name the failure mode.
    #[test]
    fn every_error_variant_has_a_display_message() {
        let store_errors = [
            StoreError::Io {
                message: "no such file".into(),
            },
            StoreError::Parse {
                line: 3,
                column: 5,
                message: "expected shape".into(),
            },
        ];
        for err in store_errors {
            assert!(!err.to_string().is_empty());
        }

        let export_errors = [
            ExportError::Vector {
                message: "boom".into(),
            },
            ExportError::Raster {
                message: "boom".into(),
            },
            ExportError::Unsupported,
        ];
        for err in export_errors {
            assert!(!err.to_string().is_empty());
        }

        let catalog_errors = [
            CatalogError::Io {
                message: "read-only fs".into(),
            },
            CatalogError::Invalid {
                message: "no anchors".into(),
            },
        ];
        for err in catalog_errors {
            assert!(!err.to_string().is_empty());
        }
    }

    /// `Parse` carries its location; `Io` does not.
    #[test]
    fn parse_errors_expose_their_location() {
        let err = StoreError::Parse {
            line: 7,
            column: 2,
            message: "x".into(),
        };
        assert_eq!(err.line_col(), Some((7, 2)));
        assert_eq!(
            StoreError::Io {
                message: "y".into()
            }
            .line_col(),
            None
        );
    }
}
