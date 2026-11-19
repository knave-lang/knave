use std::fmt;

use super::reaching_text::ReachingText;
use crate::core::PrimitiveType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Direct,
    Referenced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Representation {
    Owned,
    View,
    StaticView,
    Mixed,
    Lent,
    LentMixed,
}

impl Representation {
    pub fn written(primitive: PrimitiveType, access: Access) -> Option<Self> {
        match (primitive, access) {
            (PrimitiveType::String, Access::Direct) => Some(Self::Owned),
            (PrimitiveType::String, Access::Referenced) => Some(Self::Lent),
            (PrimitiveType::Str, _) => Some(Self::View),
            _ => None,
        }
    }

    pub fn of_stored(text: ReachingText) -> Self {
        match (
            text.holds(ReachingText::OWNED),
            text.holds(ReachingText::STATIC),
            text.is_short_lived(),
        ) {
            (false, true, false) => Self::StaticView,
            (true, true, _) => Self::Mixed,
            _ => Self::Owned,
        }
    }

    pub fn of_held(text: ReachingText) -> Self {
        match (
            text.holds(ReachingText::OWNED),
            text.holds(ReachingText::STATIC),
            text.is_short_lived(),
        ) {
            (true, true, _) => Self::Mixed,
            (true, false, _) => Self::Owned,
            (false, true, false) => Self::StaticView,
            _ => Self::View,
        }
    }

    pub fn of_local(text: ReachingText) -> Self {
        if text.is_only(ReachingText::LENT) {
            Self::Lent
        } else {
            Self::of_held(text)
        }
    }

    pub fn of_tied_parameter(variable: ReachingText) -> Self {
        match (
            variable.holds(ReachingText::OWNED),
            variable.holds(ReachingText::LENT),
            variable.holds(ReachingText::STATIC),
        ) {
            (false, _, _) => Self::View,
            (true, false, true) => Self::LentMixed,
            (true, _, _) => Self::Lent,
        }
    }

    pub fn takes_ownership(self) -> bool {
        matches!(self, Self::Owned | Self::Mixed)
    }

    pub fn refuses(self, arriving: ReachingText) -> bool {
        match self {
            Self::Owned | Self::Mixed => arriving.is_short_lived(),
            Self::Lent => arriving.holds(ReachingText::STATIC.with(ReachingText::BORROWED)),
            Self::LentMixed => arriving.holds(ReachingText::BORROWED),
            Self::View | Self::StaticView => false,
        }
    }
}

impl fmt::Display for Representation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Owned => "owned",
            Self::View => "a view",
            Self::StaticView => "a 'static view",
            Self::Mixed => "mixed",
            Self::Lent => "lent",
            Self::LentMixed => "lent mixed",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ReachingText, Representation};

    const OWNED: ReachingText = ReachingText::OWNED;
    const STATIC: ReachingText = ReachingText::STATIC;
    const BORROWED: ReachingText = ReachingText::BORROWED;
    const LENT: ReachingText = ReachingText::LENT;

    #[test]
    fn a_field_with_nothing_stored_is_owned() {
        assert_eq!(
            Representation::of_stored(ReachingText::NOTHING),
            Representation::Owned
        );
    }

    #[test]
    fn a_field_of_static_text_is_a_static_view() {
        assert_eq!(
            Representation::of_stored(STATIC),
            Representation::StaticView
        );
    }

    #[test]
    fn a_field_of_static_and_short_lived_text_is_owned() {
        assert_eq!(
            Representation::of_stored(STATIC.with(BORROWED)),
            Representation::Owned
        );
    }

    #[test]
    fn a_field_of_owned_and_static_text_is_mixed() {
        assert_eq!(
            Representation::of_stored(OWNED.with(STATIC)),
            Representation::Mixed
        );
    }

    #[test]
    fn a_local_with_nothing_stored_is_a_view() {
        assert_eq!(
            Representation::of_held(ReachingText::NOTHING),
            Representation::View
        );
    }

    #[test]
    fn a_local_of_owned_text_is_owned() {
        assert_eq!(Representation::of_held(OWNED), Representation::Owned);
    }

    #[test]
    fn a_local_of_owned_and_static_text_is_mixed() {
        assert_eq!(
            Representation::of_held(OWNED.with(STATIC)),
            Representation::Mixed
        );
    }

    #[test]
    fn a_local_of_static_text_is_a_static_view() {
        assert_eq!(Representation::of_held(STATIC), Representation::StaticView);
    }

    #[test]
    fn a_local_of_static_and_borrowed_text_is_a_view() {
        assert_eq!(
            Representation::of_held(STATIC.with(BORROWED)),
            Representation::View
        );
    }

    #[test]
    fn a_local_initialized_only_with_a_lent_value_is_lent() {
        assert_eq!(Representation::of_local(LENT), Representation::Lent);
    }

    #[test]
    fn a_local_of_lent_and_static_text_is_a_view() {
        assert_eq!(
            Representation::of_local(LENT.with(STATIC)),
            Representation::View
        );
    }

    #[test]
    fn a_local_of_static_text_alone_is_not_lent_mixed() {
        assert_eq!(Representation::of_local(STATIC), Representation::StaticView);
    }

    #[test]
    fn a_tied_parameter_without_owned_text_at_its_variable_is_a_view() {
        assert_eq!(
            Representation::of_tied_parameter(LENT.with(STATIC)),
            Representation::View
        );
    }

    #[test]
    fn a_tied_parameter_with_a_lent_caller_is_lent() {
        assert_eq!(
            Representation::of_tied_parameter(OWNED.with(LENT).with(STATIC)),
            Representation::Lent
        );
    }

    #[test]
    fn a_tied_parameter_with_static_callers_and_no_loan_is_lent_mixed() {
        assert_eq!(
            Representation::of_tied_parameter(OWNED.with(STATIC)),
            Representation::LentMixed
        );
    }

    #[test]
    fn a_tied_parameter_with_only_owned_text_at_its_variable_is_lent() {
        assert_eq!(
            Representation::of_tied_parameter(OWNED),
            Representation::Lent
        );
    }

    #[test]
    fn an_owned_slot_refuses_a_lent_value() {
        assert!(Representation::Owned.refuses(LENT));
        assert!(!Representation::Owned.refuses(STATIC));
        assert!(!Representation::View.refuses(LENT));
    }

    #[test]
    fn a_lent_parameter_refuses_static_and_borrowed_callers() {
        assert!(Representation::Lent.refuses(STATIC));
        assert!(Representation::Lent.refuses(BORROWED));
        assert!(!Representation::Lent.refuses(OWNED.with(LENT)));
    }

    #[test]
    fn a_lent_mixed_parameter_refuses_only_borrowed_callers() {
        assert!(Representation::LentMixed.refuses(BORROWED));
        assert!(!Representation::LentMixed.refuses(STATIC));
    }

    #[test]
    fn only_owned_and_mixed_slots_take_ownership() {
        assert!(Representation::Mixed.takes_ownership());
        assert!(!Representation::StaticView.takes_ownership());
    }

    #[test]
    fn written_text_reads_as_its_representation() {
        use super::Access;
        use crate::core::PrimitiveType;
        assert_eq!(
            Representation::written(PrimitiveType::Str, Access::Referenced),
            Some(Representation::View)
        );
        assert_eq!(
            Representation::written(PrimitiveType::Bool, Access::Direct),
            None
        );
    }
}
