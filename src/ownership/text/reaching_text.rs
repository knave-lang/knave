#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReachingText(u8);

impl ReachingText {
    pub const NOTHING: Self = Self(0);
    pub const OWNED: Self = Self(1);
    pub const STATIC: Self = Self(1 << 1);
    pub const BORROWED: Self = Self(1 << 2);
    pub const LENT: Self = Self(1 << 3);
    pub const ASSUMED: Self = Self(1 << 4);
    pub const SHORT_LIVED_VIEWS: Self = Self(Self::BORROWED.0 | Self::ASSUMED.0);

    pub fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub fn holds(self, kind: Self) -> bool {
        self.0 & kind.0 != 0
    }

    pub fn is_short_lived(self) -> bool {
        self.holds(Self::SHORT_LIVED_VIEWS.with(Self::LENT))
    }

    pub fn is_only(self, kinds: Self) -> bool {
        self.0 != 0 && self.without(kinds) == Self::NOTHING
    }
}

#[cfg(test)]
mod tests {
    use super::ReachingText;

    #[test]
    fn a_lent_value_is_short_lived() {
        assert!(ReachingText::LENT.is_short_lived());
    }

    #[test]
    fn an_assumed_view_is_short_lived() {
        assert!(ReachingText::ASSUMED.is_short_lived());
    }

    #[test]
    fn static_text_is_not_short_lived() {
        assert!(!ReachingText::STATIC.is_short_lived());
    }

    #[test]
    fn nothing_is_only_nothing_of_any_kind() {
        assert!(!ReachingText::NOTHING.is_only(ReachingText::LENT));
    }

    #[test]
    fn lent_and_static_text_is_only_those_kinds() {
        let lent_mixed = ReachingText::LENT.with(ReachingText::STATIC);
        assert!(lent_mixed.is_only(ReachingText::LENT.with(ReachingText::STATIC)));
        assert!(!lent_mixed.is_only(ReachingText::LENT));
    }
}
