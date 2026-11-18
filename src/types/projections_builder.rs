use std::rc::Rc;

use super::lowered_associated_value::LoweredAssociatedValue;
use super::projection::{AssociatedType, ProjectionTable};
use super::trait_rows_reach::Implementor;
use super::traits::TraitTable;
use crate::core::BindingId;

pub(super) struct ProjectionsBuilder<'a> {
    table: &'a mut TraitTable,
}

impl<'a> ProjectionsBuilder<'a> {
    pub(super) fn new(table: &'a mut TraitTable) -> Self {
        Self { table }
    }

    pub(super) fn declare(self, binding: BindingId, associated: AssociatedType) {
        self.table.projections_mut().declare(binding, associated);
    }

    pub(super) fn add_rules(self, implementor: &Implementor, values: Vec<LoweredAssociatedValue>) {
        let projections = self.table.projections_mut();
        for value in values {
            projections.add_rule(value.projection, implementor, value.value);
        }
    }

    pub(super) fn share(self) -> Rc<ProjectionTable> {
        self.table.share_projections()
    }
}
