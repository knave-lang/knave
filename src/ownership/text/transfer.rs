use super::reaching_text::ReachingText;
use super::representation::Access;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transfer {
    Move,
    Refer,
    View,
}

impl Transfer {
    pub fn through(access: Access) -> Self {
        match access {
            Access::Direct => Self::Move,
            Access::Referenced => Self::Refer,
        }
    }

    pub fn carry(self, text: ReachingText) -> ReachingText {
        match self {
            Self::Move => text,
            Self::Refer => Self::replace(text, ReachingText::OWNED, ReachingText::LENT),
            Self::View => Self::replace(
                text,
                ReachingText::OWNED.with(ReachingText::LENT),
                ReachingText::BORROWED,
            ),
        }
    }

    pub fn then(self, next: Self) -> Self {
        match (self, next) {
            (Self::Move, other) | (other, Self::Move) => other,
            (Self::View, _) | (_, Self::View) => Self::View,
            (Self::Refer, Self::Refer) => Self::Refer,
        }
    }

    fn replace(
        text: ReachingText,
        replaced: ReachingText,
        replacement: ReachingText,
    ) -> ReachingText {
        let kept = text.without(replaced);
        if text.holds(replaced) {
            kept.with(replacement)
        } else {
            kept
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReachingText, Transfer};

    #[test]
    fn a_reference_to_owned_text_is_lent() {
        assert_eq!(
            Transfer::Refer.carry(ReachingText::OWNED),
            ReachingText::LENT
        );
    }

    #[test]
    fn a_reference_keeps_static_text_static() {
        assert_eq!(
            Transfer::Refer.carry(ReachingText::STATIC),
            ReachingText::STATIC
        );
    }

    #[test]
    fn a_view_of_owned_text_is_borrowed() {
        assert_eq!(
            Transfer::View.carry(ReachingText::OWNED),
            ReachingText::BORROWED
        );
    }

    #[test]
    fn a_view_of_a_lent_value_is_borrowed() {
        assert_eq!(
            Transfer::View.carry(ReachingText::LENT),
            ReachingText::BORROWED
        );
    }

    #[test]
    fn a_move_carries_text_unchanged() {
        assert_eq!(
            Transfer::Move.carry(ReachingText::OWNED),
            ReachingText::OWNED
        );
    }

    #[test]
    fn a_view_outranks_a_reference() {
        assert_eq!(Transfer::Refer.then(Transfer::View), Transfer::View);
        assert_eq!(Transfer::View.then(Transfer::Refer), Transfer::View);
    }

    #[test]
    fn a_move_leaves_the_other_transfer() {
        assert_eq!(Transfer::Move.then(Transfer::Refer), Transfer::Refer);
        assert_eq!(Transfer::Refer.then(Transfer::Move), Transfer::Refer);
    }

    #[test]
    fn two_references_stay_a_reference() {
        assert_eq!(Transfer::Refer.then(Transfer::Refer), Transfer::Refer);
    }
}
