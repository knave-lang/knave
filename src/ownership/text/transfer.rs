use super::access::Access;
use super::reaching_text::ReachingText;
use crate::types::Borrow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transfer {
    Move,
    Refer,
    Lend,
    View,
}

impl Transfer {
    pub fn through(access: Access) -> Self {
        match access {
            Access::Direct => Self::Move,
            Access::Referenced => Self::Refer,
        }
    }

    pub fn passing(borrow: &Borrow) -> Self {
        match borrow {
            Borrow::Deref(_) => Self::Lend,
            _ => Self::Move,
        }
    }

    pub fn lending(access: Access) -> Self {
        match access {
            Access::Direct => Self::Move,
            Access::Referenced => Self::Lend,
        }
    }

    pub fn lends(self) -> bool {
        matches!(self, Self::Refer | Self::Lend)
    }

    pub fn without_referent(self) -> Self {
        match self {
            Self::Lend => Self::Refer,
            other => other,
        }
    }

    pub fn carry(self, text: ReachingText) -> ReachingText {
        match self {
            Self::Move => text,
            Self::Refer => Self::replace(text, ReachingText::OWNED, ReachingText::LENT),
            Self::Lend => Self::replace(
                text,
                ReachingText::OWNED,
                ReachingText::OWNED.with(ReachingText::LENT),
            ),
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
            (Self::Lend, _) | (_, Self::Lend) => Self::Lend,
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
    fn a_loan_of_owned_text_keeps_its_owned_referent() {
        assert_eq!(
            Transfer::Lend.carry(ReachingText::OWNED),
            ReachingText::OWNED.with(ReachingText::LENT)
        );
        assert_eq!(
            Transfer::Lend.carry(ReachingText::STATIC),
            ReachingText::STATIC
        );
    }

    #[test]
    fn a_loan_of_a_reference_is_a_loan() {
        assert_eq!(Transfer::Refer.then(Transfer::Lend), Transfer::Lend);
        assert_eq!(Transfer::Lend.then(Transfer::Refer), Transfer::Lend);
        assert_eq!(Transfer::Lend.then(Transfer::View), Transfer::View);
    }

    #[test]
    fn a_loan_without_its_referent_is_a_reference() {
        assert_eq!(Transfer::Lend.without_referent(), Transfer::Refer);
        assert_eq!(Transfer::Move.without_referent(), Transfer::Move);
    }

    #[test]
    fn references_and_loans_lend_and_moves_do_not() {
        assert!(Transfer::Lend.lends());
        assert!(Transfer::Refer.lends());
        assert!(!Transfer::Move.lends());
        assert!(!Transfer::View.lends());
    }

    #[test]
    fn a_referenced_argument_is_lent_and_a_direct_one_moved() {
        use super::Access;
        assert_eq!(Transfer::lending(Access::Referenced), Transfer::Lend);
        assert_eq!(Transfer::lending(Access::Direct), Transfer::Move);
    }

    #[test]
    fn a_dereferenced_argument_is_lent_and_any_other_moved() {
        use crate::types::Borrow;
        assert_eq!(Transfer::passing(&Borrow::Deref(1)), Transfer::Lend);
        assert_eq!(Transfer::passing(&Borrow::Shared), Transfer::Move);
    }

    #[test]
    fn two_references_stay_a_reference() {
        assert_eq!(Transfer::Refer.then(Transfer::Refer), Transfer::Refer);
    }
}
