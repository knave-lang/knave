use crate::types::{Borrow, Type, TypedArgument, TypedExpr, TypedExprKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArgumentRole {
    Receiver,
    FieldOwner,
    IndexReceiver,
    Borrowed,
    MutPlace,
    Coerced,
    Ordinary,
}

impl ArgumentRole {
    const METHOD_CALL_OPENERS: [char; 2] = ['(', ':'];

    pub(super) fn receiving(member: &str) -> Self {
        let name = member.strip_prefix('.').unwrap_or(member);
        let after_name = name
            .trim_start_matches(|character: char| character.is_alphanumeric() || character == '_');
        if after_name
            .trim_start()
            .starts_with(Self::METHOD_CALL_OPENERS)
        {
            ArgumentRole::Receiver
        } else {
            ArgumentRole::FieldOwner
        }
    }

    pub(super) fn reads_a_member(self) -> bool {
        matches!(self, ArgumentRole::Receiver | ArgumentRole::FieldOwner)
    }

    pub(super) fn writes_borrow_prefix(self, argument: &TypedArgument) -> bool {
        matches!(argument.borrow, Borrow::Shared | Borrow::Mutable)
            && !self.borrowed_already(argument)
    }

    pub(super) fn borrow_prefix(self, argument: &TypedArgument) -> Option<String> {
        match self {
            ArgumentRole::MutPlace => None,
            _ => self.written_prefix(argument),
        }
    }

    fn written_prefix(self, argument: &TypedArgument) -> Option<String> {
        let borrowed_already = self.borrowed_already(argument);
        match argument.borrow {
            Borrow::Shared if !borrowed_already => Some("&".to_string()),
            Borrow::Mutable if !borrowed_already => Some("&mut ".to_string()),
            Borrow::Deref(_) | Borrow::Collapse(_) => self
                .dereferenced_layers(argument)
                .map(|levels| "*".repeat(levels)),
            _ => None,
        }
    }

    pub(super) fn dereferenced_layers(self, argument: &TypedArgument) -> Option<usize> {
        let autodereferences = matches!(self, ArgumentRole::FieldOwner | ArgumentRole::Coerced);
        match argument.borrow {
            Borrow::Deref(levels) => Some(levels),
            _ => argument
                .collapsed_shared_layers()
                .filter(|_| !autodereferences),
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
                ArgumentRole::Receiver | ArgumentRole::FieldOwner | ArgumentRole::IndexReceiver,
                Borrow::Shared | Borrow::Mutable,
            ) | (ArgumentRole::Borrowed, Borrow::Shared)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_called_with_arguments_receives_a_method_call() {
        for member in [".len()", ".max(", ".max (", ".collect::<Vec<_>>()"] {
            assert_eq!(
                ArgumentRole::receiving(member),
                ArgumentRole::Receiver,
                "{member}"
            );
        }
    }

    #[test]
    fn a_member_without_arguments_reads_a_field() {
        for member in [".radius", ".1", ".blank.then_some("] {
            assert_eq!(
                ArgumentRole::receiving(member),
                ArgumentRole::FieldOwner,
                "{member}"
            );
        }
    }
}
