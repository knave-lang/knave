use super::read_view::ReadView;
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
        let open = Bound {
            argument: None,
            ..bound.clone()
        };
        match traits
            .rows(bound.binding)
            .select(&open, self.owner(bound, implementor))
        {
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

    fn owner<'t>(&self, bound: &Bound, implementor: &'t Type) -> &'t Type {
        if Self::is_written(bound) {
            implementor
        } else {
            Unborrowed::of(implementor)
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
        let met = !Self::is_written(bound) || ReadView::of(&read).is_written_as(&expected);
        met.then_some(expected)
    }

    pub(super) fn settles_later(traits: &TraitTable, bound: &Bound) -> bool {
        let reads_field = (traits.get(bound.binding)).is_some_and(|known| known.reads_field);
        reads_field && !Self::is_written(bound)
    }

    fn is_written(bound: &Bound) -> bool {
        bound.origins.is_written()
    }
}
