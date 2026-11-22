use super::selection::Selection;
use super::traits::TraitTable;
use super::ty::{Bound, Type};
use super::unborrowed::Unborrowed;
use super::unifier::Unifier;

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
        match Self::open_selection(traits, bound, implementor) {
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

    pub(super) fn open_selection(
        traits: &TraitTable,
        bound: &Bound,
        implementor: &Type,
    ) -> Selection {
        let open = Bound {
            argument: None,
            ..bound.clone()
        };
        traits
            .rows(bound.binding)
            .select(&open, Unborrowed::of(implementor))
    }

    pub(super) fn owner<'t>(traits: &TraitTable, bound: &Bound, implementor: &'t Type) -> &'t Type {
        if Self::reads_field(traits, bound) {
            Unborrowed::of(implementor)
        } else {
            implementor
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
        (expected == read || expected.has_vars()).then_some(read)
    }

    pub(super) fn reads_open_field(traits: &TraitTable, bound: &Bound) -> bool {
        let open = (bound.argument.as_ref()).is_some_and(Type::has_vars);
        Self::reads_field(traits, bound) && open
    }

    pub(super) fn reads_field(traits: &TraitTable, bound: &Bound) -> bool {
        (traits.get(bound.binding)).is_some_and(|known| known.reads_field)
    }
}
