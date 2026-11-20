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
            .select(&open, Unborrowed::of(implementor))
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

    pub(super) fn owner<'t>(traits: &TraitTable, bound: &Bound, implementor: &'t Type) -> &'t Type {
        let reads_field = (traits.get(bound.binding)).is_some_and(|known| known.reads_field);
        if reads_field {
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
        if expected == read {
            return Some(read);
        }
        if expected.has_vars() {
            return Some(ReadView::of(&read).at_depth_of(&expected));
        }
        let view = ReadView::of(&read);
        let reached = !Self::is_written(bound) && view.is_reached_as(&expected);
        (view.is_written_as(&expected) || reached).then_some(expected)
    }

    pub(super) fn is_written(bound: &Bound) -> bool {
        bound.origins.is_written()
    }
}
