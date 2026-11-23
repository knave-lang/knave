use super::access::Access;
use super::reaching_text::ReachingText;
use crate::core::BindingId;
use crate::types::Borrow;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Transfer {
    Move,
    Refer,
    Reborrow,
    Lend,
    View,
    Query,
    Through(BindingId),
}

impl Transfer {
    pub fn referring(access: Access) -> Self {
        match access {
            Access::Direct => Self::Move,
            Access::Referenced | Access::Mutable => Self::Refer,
        }
    }

    pub fn passing(borrow: &Borrow) -> Self {
        match borrow {
            Borrow::Deref(_) => Self::Reborrow,
            _ => Self::Move,
        }
    }

    pub fn lending(access: Access) -> Self {
        match access {
            Access::Direct => Self::Move,
            Access::Referenced | Access::Mutable => Self::Lend,
        }
    }

    pub fn lends(self) -> bool {
        matches!(self, Self::Refer | Self::Reborrow | Self::Lend)
    }

    pub fn without_referent(self) -> Self {
        match self {
            Self::Reborrow | Self::Lend => Self::Refer,
            other => other,
        }
    }

    pub fn carry(self, text: ReachingText) -> ReachingText {
        match self {
            Self::Move => text,
            Self::Refer => Self::referred(text),
            Self::Reborrow => Self::replace(
                text,
                ReachingText::OWNED,
                ReachingText::OWNED.with(ReachingText::LENT),
            ),
            Self::Lend => Self::replace(
                text,
                ReachingText::OWNED.with(ReachingText::LENT),
                ReachingText::OWNED.with(ReachingText::LENT),
            ),
            Self::Query => ReachingText::NOTHING,
            Self::View | Self::Through(_) => Self::replace(
                text,
                ReachingText::OWNED
                    .with(ReachingText::LENT)
                    .with(ReachingText::LENT_MIXED),
                ReachingText::BORROWED,
            ),
        }
    }

    fn referred(text: ReachingText) -> ReachingText {
        let mixed = ReachingText::OWNED.with(ReachingText::STATIC);
        if text.holds(ReachingText::OWNED) && text.holds(ReachingText::STATIC) {
            text.without(mixed).with(ReachingText::LENT_MIXED)
        } else {
            Self::replace(text, ReachingText::OWNED, ReachingText::LENT)
        }
    }

    pub fn then(self, next: Self) -> Self {
        self.max(next)
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
    fn a_reference_to_mixed_text_is_lent_mixed() {
        let mixed = ReachingText::OWNED.with(ReachingText::STATIC);
        assert_eq!(Transfer::Refer.carry(mixed), ReachingText::LENT_MIXED);
        assert_eq!(
            Transfer::View.carry(ReachingText::LENT_MIXED),
            ReachingText::BORROWED
        );
    }

    #[test]
    fn a_query_carries_no_text() {
        assert_eq!(
            Transfer::Query.carry(ReachingText::STATIC),
            ReachingText::NOTHING
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
    fn a_loan_of_a_lent_value_lends_its_owned_referent() {
        assert_eq!(
            Transfer::Lend.carry(ReachingText::LENT),
            ReachingText::OWNED.with(ReachingText::LENT)
        );
    }

    #[test]
    fn a_reborrow_keeps_a_lent_value_lent() {
        assert_eq!(
            Transfer::Reborrow.carry(ReachingText::LENT),
            ReachingText::LENT
        );
        assert_eq!(
            Transfer::Reborrow.carry(ReachingText::OWNED),
            ReachingText::OWNED.with(ReachingText::LENT)
        );
        assert!(Transfer::Reborrow.lends());
        assert_eq!(Transfer::Reborrow.without_referent(), Transfer::Refer);
        assert_eq!(Transfer::Reborrow.then(Transfer::Lend), Transfer::Lend);
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
        assert_eq!(Transfer::lending(Access::Mutable), Transfer::Lend);
        assert_eq!(Transfer::referring(Access::Mutable), Transfer::Refer);
    }

    #[test]
    fn a_dereferenced_argument_is_reborrowed_and_any_other_moved() {
        use crate::types::Borrow;
        assert_eq!(Transfer::passing(&Borrow::Deref(1)), Transfer::Reborrow);
        assert_eq!(Transfer::passing(&Borrow::Shared), Transfer::Move);
    }

    #[test]
    fn two_references_stay_a_reference() {
        assert_eq!(Transfer::Refer.then(Transfer::Refer), Transfer::Refer);
    }
}
