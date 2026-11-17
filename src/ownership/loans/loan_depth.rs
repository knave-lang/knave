#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LoanDepth(usize);

impl LoanDepth {
    pub const TOP: Self = Self(0);
    pub const DEEPEST: Self = Self(7);

    pub fn at(index: usize) -> Self {
        Self(index.min(Self::DEEPEST.0))
    }

    pub fn shifted(self, offset: isize) -> Option<Self> {
        match self {
            Self::DEEPEST => Some(self),
            _ => self.0.checked_add_signed(offset).map(Self::at),
        }
    }

    pub fn index(self) -> usize {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::LoanDepth;

    #[test]
    fn a_depth_past_the_deepest_is_held_at_the_deepest() {
        assert_eq!(
            LoanDepth::at(LoanDepth::DEEPEST.index() + 3),
            LoanDepth::DEEPEST
        );
        assert_eq!(LoanDepth::at(6).shifted(4), Some(LoanDepth::DEEPEST));
    }

    #[test]
    fn the_deepest_depth_stays_deepest_under_any_referent() {
        assert_eq!(LoanDepth::DEEPEST.shifted(-5), Some(LoanDepth::DEEPEST));
    }

    #[test]
    fn a_referent_of_the_top_depth_holds_nothing() {
        assert_eq!(LoanDepth::TOP.shifted(-1), None);
        assert_eq!(LoanDepth::at(2).shifted(-1), Some(LoanDepth::at(1)));
    }
}
