//! Parse errors with 1-based line and column positions (S-004 AC1).

use core::fmt;

/// A parse failure at a specific position in the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based character column where the problem starts.
    pub column: usize,
    /// 1-based physical line number where the problem was detected.
    pub line: usize,
    /// Human-readable description of the problem.
    pub message: String,
}

impl ParseError {
    /// Builds an error at `line`/`column` (both 1-based) with a free-form message.
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

impl core::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::ParseError;

    #[test]
    fn display_shows_line_column_and_message() {
        let err = ParseError::new(3, 14, "boom");
        assert_eq!(err.to_string(), "line 3, column 14: boom");
    }
}
