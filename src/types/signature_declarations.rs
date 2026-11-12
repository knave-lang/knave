use rustc_hash::FxHashMap;

use crate::core::{BindingArena, BindingId};

#[derive(Default)]
pub struct SignatureDeclarations {
    pub(super) prelude_definitions: Vec<BindingId>,
    pub(super) written_parameters: FxHashMap<BindingId, Vec<bool>>,
}

impl SignatureDeclarations {
    pub fn prelude_defines_value(&self, name: &str, bindings: &BindingArena) -> bool {
        self.prelude_definitions
            .iter()
            .any(|binding| bindings.get(*binding).name() == name)
    }

    pub fn written_at(&self, callee: BindingId, position: usize) -> bool {
        self.written_parameters
            .get(&callee)
            .and_then(|written| written.get(position))
            .copied()
            .unwrap_or(true)
    }

    pub fn written_parameter_count(&self, callee: BindingId) -> usize {
        self.written_parameters.get(&callee).map_or(0, Vec::len)
    }
}
