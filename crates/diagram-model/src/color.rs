//! Diagram colors (FR15): the three presentation roles a diagram can carry —
//! background, shape fill and line/text — as pure value data.
//!
//! Colors are *presentation* properties of the aggregate: they never affect grid
//! resolution (the render pipeline is a pure function of the objects), they are
//! stored with the diagram in the `.ad` document (v1.1) and map 1:1 onto the SVG
//! exporter's settings slots (CR-009). An unset role means "use the medium's
//! default" — the canvas surface, svgbob's black-on-white, etc.

/// A failure to parse a `#rrggbb` hex color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorParseError {
    /// The input was empty or lacked the leading `#`.
    MissingHash,
    /// The body is not exactly six characters long.
    BadLength {
        /// How many characters were present (after the `#`).
        found: usize,
    },
    /// A character in the body is not a hexadecimal digit.
    BadDigit {
        /// The offending character.
        ch: char,
    },
}

impl std::fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHash => write!(f, "expected a #rrggbb color"),
            Self::BadLength { found } => {
                write!(f, "expected 6 hex digits after '#', found {found}")
            }
            Self::BadDigit { ch } => write!(f, "'{ch}' is not a hexadecimal digit"),
        }
    }
}

impl std::error::Error for ColorParseError {}

/// An opaque sRGB color in 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// Red, 0–255.
    pub r: u8,
    /// Green, 0–255.
    pub g: u8,
    /// Blue, 0–255.
    pub b: u8,
}

impl Rgb {
    /// Builds a color from raw channels (no validation needed — `u8` bounds them).
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#rrggbb` (case-insensitive; the leading `#` is required).
    pub fn from_hex(s: &str) -> Result<Self, ColorParseError> {
        let hex = s.strip_prefix('#').ok_or(ColorParseError::MissingHash)?;
        if hex.len() != 6 {
            return Err(ColorParseError::BadLength { found: hex.len() });
        }
        let (pairs, _) = hex.as_bytes().as_chunks::<2>(); // len == 6 checked above
        let mut bytes = [0u8; 3];
        for (i, pair) in pairs.iter().enumerate() {
            match u8::from_str_radix(
                std::str::from_utf8(pair).expect("checked byte pairs are utf-8"),
                16,
            ) {
                Ok(v) => bytes[i] = v,
                Err(_) => {
                    let ch = pair
                        .iter()
                        .find(|c| !c.is_ascii_hexdigit())
                        .map(|c| *c as char)
                        .unwrap_or('#');
                    return Err(ColorParseError::BadDigit { ch });
                }
            }
        }
        Ok(Self {
            r: bytes[0],
            g: bytes[1],
            b: bytes[2],
        })
    }

    /// The canonical lowercase `#rrggbb` form — the only spelling the `.ad` writer emits.
    #[must_use]
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// The three color roles of a diagram (FR15). The names double as the `.ad` v1.1
/// statement tokens — one spelling in both places by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRole {
    /// The canvas / SVG document background.
    Background,
    /// The interior of shape bodies (beneath their glyphs).
    Fill,
    /// Box-drawing characters and text — everything the grid renders as ink.
    LineText,
}

impl ColorRole {
    /// All roles in canonical `.ad` order (the writer's emission order).
    pub const ALL: [Self; 3] = [Self::Background, Self::Fill, Self::LineText];

    /// The role's token in the `colors` statement (`background` / `fill` / `lines`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Fill => "fill",
            Self::LineText => "lines",
        }
    }

    /// The role's display name in the GUI color dialog (FR15 / CR-009 AC5).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Background => "Background",
            Self::Fill => "Shape fill",
            Self::LineText => "Lines & text",
        }
    }

    /// Parses a `colors` statement token; `None` for anything that is not a role.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "background" => Some(Self::Background),
            "fill" => Some(Self::Fill),
            "lines" => Some(Self::LineText),
            _ => None,
        }
    }
}

/// The diagram's presentation colors: one optional color per role. All-unset is the
/// default — every medium falls back to its own defaults (FR15 / CR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagramColors {
    /// The canvas / document background (unset = medium default).
    pub background: Option<Rgb>,
    /// Shape body fill (unset = no fill — the medium's default ink-on-surface look).
    pub fill: Option<Rgb>,
    /// Line & text ink (unset = medium default).
    pub line_text: Option<Rgb>,
}

impl DiagramColors {
    /// The all-unset value — const-constructible so it can seed [`crate::Diagram::new`].
    pub(crate) const NONE: Self = Self {
        background: None,
        fill: None,
        line_text: None,
    };

    /// The color for `role`, if set.
    #[must_use]
    pub const fn get(self, role: ColorRole) -> Option<Rgb> {
        match role {
            ColorRole::Background => self.background,
            ColorRole::Fill => self.fill,
            ColorRole::LineText => self.line_text,
        }
    }

    /// Sets (or clears, with `None`) exactly one role; the others are untouched.
    pub fn set(&mut self, role: ColorRole, color: Option<Rgb>) {
        match role {
            ColorRole::Background => self.background = color,
            ColorRole::Fill => self.fill = color,
            ColorRole::LineText => self.line_text = color,
        }
    }

    /// Whether any role is set — the writer emits the `colors` statement iff this.
    #[must_use]
    pub const fn has_any(self) -> bool {
        self.background.is_some() || self.fill.is_some() || self.line_text.is_some()
    }
}

/// All roles unset: every medium falls back to its own defaults (FR15).
impl Default for DiagramColors {
    fn default() -> Self {
        Self::NONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip_is_canonical_lowercase() {
        for (input, expected) in [
            ("#ff0000", Rgb::new(255, 0, 0)),
            ("#FF8800", Rgb::new(255, 136, 0)),
            ("#0a1B2C", Rgb::new(10, 27, 44)),
            ("#000000", Rgb::new(0, 0, 0)),
            ("#ffffff", Rgb::new(255, 255, 255)),
        ] {
            let rgb = Rgb::from_hex(input).expect("parses");
            assert_eq!(rgb, expected, "input {input}");
            assert_eq!(
                rgb.to_hex(),
                input.to_lowercase(),
                "canonical form of {input}"
            );
        }
    }

    #[test]
    fn hex_errors_are_specific() {
        assert_eq!(Rgb::from_hex(""), Err(ColorParseError::MissingHash));
        assert_eq!(Rgb::from_hex("ff0000"), Err(ColorParseError::MissingHash));
        assert_eq!(
            Rgb::from_hex("#fff"),
            Err(ColorParseError::BadLength { found: 3 })
        );
        assert_eq!(
            Rgb::from_hex("#ffffff1"),
            Err(ColorParseError::BadLength { found: 7 })
        );
        assert_eq!(
            Rgb::from_hex("#0g0000"),
            Err(ColorParseError::BadDigit { ch: 'g' })
        );
    }

    #[test]
    fn role_tokens_round_trip() {
        for role in ColorRole::ALL {
            assert_eq!(ColorRole::parse(role.name()), Some(role));
        }
        assert_eq!(ColorRole::parse("border"), None);
        assert_eq!(ColorRole::parse("BACKGROUND"), None, "tokens are exact");
    }

    #[test]
    fn colors_default_is_all_unset_and_set_touches_one_role() {
        let mut c = DiagramColors::default();
        assert!(!c.has_any());
        c.set(ColorRole::Fill, Some(Rgb::new(1, 2, 3)));
        assert!(c.has_any());
        assert_eq!(c.get(ColorRole::Fill), Some(Rgb::new(1, 2, 3)));
        assert_eq!(c.get(ColorRole::Background), None);
        assert_eq!(c.get(ColorRole::LineText), None);
        // Setting another role leaves the first alone; clearing works.
        c.set(ColorRole::Background, Some(Rgb::new(9, 9, 9)));
        assert_eq!(c.get(ColorRole::Fill), Some(Rgb::new(1, 2, 3)));
        c.set(ColorRole::Fill, None);
        assert!(!c.get(ColorRole::Fill).is_some());
        assert!(c.has_any(), "background still set");
    }
}
