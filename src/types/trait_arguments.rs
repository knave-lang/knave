use super::traits::TraitTable;
use crate::core::BindingId;

pub(super) struct TraitArguments<'a> {
    traits: &'a TraitTable,
}

impl<'a> TraitArguments<'a> {
    pub(super) fn of(traits: &'a TraitTable) -> Self {
        Self { traits }
    }

    pub(super) fn fixes(&self, binding: BindingId) -> bool {
        (self.traits.get(binding)).is_some_and(|rust_trait| rust_trait.associated.is_some())
    }

    pub(super) fn links(&self, binding: BindingId) -> bool {
        self.traits.get(binding).is_none_or(|rust_trait| {
            let rows = rust_trait.rows().reached().implementors();
            !self.declares_projection(binding) && rows.iter().any(|row| row.argument.is_none())
        })
    }

    fn declares_projection(&self, binding: BindingId) -> bool {
        !self.traits.projections().declared_by(binding).is_empty()
    }
}
