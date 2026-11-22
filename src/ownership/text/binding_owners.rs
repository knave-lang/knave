use rustc_hash::FxHashSet;

use super::slot_owner::SlotOwner;
use crate::core::BindingId;
use crate::types::{Type, TypedExpr, TypedExprKind, TypedLeaf};

#[derive(Default)]
pub struct BindingOwners {
    parameters: Vec<BindingId>,
    closure_parameters: FxHashSet<BindingId>,
}

impl BindingOwners {
    pub fn add_parameter(&mut self, binding: BindingId) {
        self.parameters.push(binding);
    }

    pub fn add_closure_parameters(&mut self, bindings: impl Iterator<Item = BindingId>) {
        self.closure_parameters.extend(bindings);
    }

    pub fn parameters(&self) -> &[BindingId] {
        &self.parameters
    }

    pub fn writes_a_parameter(&self, argument: &TypedExpr) -> bool {
        match &argument.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => {
                matches!(argument.ty, Type::Mut(..))
                    && matches!(self.owner(*binding), SlotOwner::Parameter(_))
            }
            _ => false,
        }
    }

    pub fn owner(&self, binding: BindingId) -> SlotOwner {
        if self.parameters.contains(&binding) || self.closure_parameters.contains(&binding) {
            SlotOwner::Parameter(binding)
        } else {
            SlotOwner::Local(binding)
        }
    }
}
