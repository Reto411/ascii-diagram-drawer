//! N/E/S/W edge connection sets — the input to glyph resolution (C1).

/// The set of edge directions a cell is connected in. A cell can connect north,
/// east, south and/or west; the resolve stage maps this set plus content to exactly
/// one glyph ([glyph-resolution](../../../doc/arc42/12-concepts/glyph-resolution.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EdgeSet(u8);

impl EdgeSet {
    /// No connections.
    pub const NONE: Self = Self(0);
    /// Connected north (up).
    pub const N: Self = Self(1 << 0);
    /// Connected east (right).
    pub const E: Self = Self(1 << 1);
    /// Connected south (down).
    pub const S: Self = Self(1 << 2);
    /// Connected west (left).
    pub const W: Self = Self(1 << 3);
    /// All four directions.
    pub const ALL: Self = Self(0b1111);

    /// Whether this set contains every direction of `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// The number of directions in the set (0..=4).
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for EdgeSet {
    type Output = Self;

    /// Union of two connection sets (structural merges are commutative).
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for EdgeSet {
    /// Unions `rhs` into this set.
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::EdgeSet;

    #[test]
    fn direction_bits_are_distinct() {
        for a in [EdgeSet::N, EdgeSet::E, EdgeSet::S, EdgeSet::W] {
            for b in [EdgeSet::N, EdgeSet::E, EdgeSet::S, EdgeSet::W] {
                assert_eq!(a.contains(b), a == b, "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn union_and_count() {
        let both = EdgeSet::N | EdgeSet::S;
        assert_eq!(both.count(), 2);
        assert!(both.contains(EdgeSet::N));
        assert!(both.contains(EdgeSet::S));
        assert!(!both.contains(EdgeSet::E));
        assert_eq!(
            (EdgeSet::N | EdgeSet::E) | (EdgeSet::S | EdgeSet::W),
            EdgeSet::ALL
        );
    }

    #[test]
    fn is_empty() {
        assert!(EdgeSet::NONE.is_empty());
        assert!(!EdgeSet::N.is_empty());
        assert!(!(EdgeSet::N | EdgeSet::W).is_empty());
    }
}
