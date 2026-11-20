use super::selection::Selection;
use super::traits::TraitTable;
use super::ty::{Bound, Type};
use super::unifier::Unifier;
use crate::core::PrimitiveType;

pub(super) struct FieldReadSelection<'u> {
    unifier: &'u Unifier,
}

impl<'u> FieldReadSelection<'u> {
    pub(super) fn new(unifier: &'u Unifier) -> Self {
        Self { unifier }
    }

    pub(super) fn select(
        &self,
        traits: &TraitTable,
        bound: &Bound,
        implementor: &Type,
    ) -> Selection {
        let open = Bound {
            argument: None,
            ..bound.clone()
        };
        match traits.rows(bound.binding).select(&open, implementor) {
            Selection::Implemented {
                requirements,
                argument: Some(read),
            } => match self.met_read(read, bound) {
                Some(argument) => Selection::Implemented {
                    requirements,
                    argument: Some(argument),
                },
                None => Selection::Missing,
            },
            other => other,
        }
    }

    fn met_read(&self, read: Type, bound: &Bound) -> Option<Type> {
        let Some(expected) = bound
            .argument
            .as_ref()
            .map(|argument| self.unifier.resolve(argument))
        else {
            return Some(read);
        };
        if expected.has_vars() || expected == read {
            return Some(read);
        }
        Self::adjusts_to(&read, &expected).then_some(expected)
    }

    fn adjusts_to(read: &Type, expected: &Type) -> bool {
        let read = Self::viewed(Self::without_borrows(read));
        read == Self::viewed(Self::without_borrows(expected))
    }

    fn without_borrows(value: &Type) -> &Type {
        match value {
            Type::Ref(_, inner) | Type::Mut(_, inner) => Self::without_borrows(inner),
            _ => value,
        }
    }

    fn viewed(value: &Type) -> Type {
        match value {
            Type::Primitive(PrimitiveType::String) => Type::Primitive(PrimitiveType::Str),
            other => other.clone(),
        }
    }
}
