use std::rc::Rc;

use crate::core::{BindingArena, BindingId, ModuleId, Namespace};
use crate::reader::Symbol;
use crate::scope::{BindingTable, Phase, Resolution};

#[derive(Clone, Copy)]
pub(super) struct TermResolver<'t> {
    table: &'t BindingTable,
    bindings: &'t BindingArena,
    phase: Phase,
}

impl<'t> TermResolver<'t> {
    pub(super) fn new(table: &'t BindingTable, bindings: &'t BindingArena, phase: Phase) -> Self {
        Self {
            table,
            bindings,
            phase,
        }
    }

    pub(super) fn resolve(&self, symbol: &Symbol, namespace: Namespace) -> Option<BindingId> {
        match self.table.resolve(symbol, namespace, self.phase) {
            Resolution::Bound(binding) => Some(binding),
            Resolution::Unbound | Resolution::Ambiguous(_) => None,
        }
    }

    pub(super) fn definition_module(&self, binding: BindingId) -> Option<ModuleId> {
        self.bindings.get(binding).kind().defining_module()
    }

    pub(super) fn visible_bindings(
        &self,
        symbol: &Symbol,
        namespace: Namespace,
    ) -> Vec<(Rc<str>, BindingId)> {
        self.table
            .visible_name_bindings(symbol, namespace, self.phase)
    }
}
