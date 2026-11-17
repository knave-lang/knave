use crate::types::{Borrow, Type, TypedArgument, TypedExpr, TypedExprKind};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ArgumentRole {
    Receiver,
    IndexReceiver,
    Borrowed,
    MutPlace,
    Ordinary,
}

impl ArgumentRole {
    pub(super) fn writes_borrow_prefix(self, argument: &TypedArgument) -> bool {
        matches!(argument.borrow, Borrow::Shared | Borrow::Mutable)
            && !self.borrowed_already(argument)
    }

    pub(super) fn borrow_prefix(self, argument: &TypedArgument) -> Option<String> {
        let borrowed_already = self.borrowed_already(argument);
        match argument.borrow {
            Borrow::Shared if !borrowed_already => Some("&".to_string()),
            Borrow::Mutable if !borrowed_already => Some("&mut ".to_string()),
            Borrow::Deref(levels) => Some("*".repeat(levels)),
            Borrow::Reborrow => None,
            Borrow::Pending(_) => {
                unreachable!("the type checker decides every pending borrow")
            }
            _ => None,
        }
    }

    pub(super) fn reborrowed_reference(self, argument: &TypedArgument) -> Option<&TypedExpr> {
        match &argument.value.kind {
            TypedExprKind::Deref(reference)
                if argument.borrow == Borrow::Shared
                    && !self.borrows_by_itself(argument.borrow)
                    && matches!(reference.ty, Type::Ref(..)) =>
            {
                Some(reference)
            }
            _ => None,
        }
    }

    fn borrowed_already(self, argument: &TypedArgument) -> bool {
        argument.value.ty.is_reference() || self.borrows_by_itself(argument.borrow)
    }

    fn borrows_by_itself(self, borrow: Borrow) -> bool {
        matches!(
            (self, borrow),
            (
                ArgumentRole::Receiver | ArgumentRole::IndexReceiver,
                Borrow::Shared | Borrow::Mutable,
            ) | (ArgumentRole::Borrowed, Borrow::Shared)
        )
    }
}
