use rustc_hash::FxHashSet;

use crate::core::{BindingId, ModuleId};

pub struct CompileTimeRoots {
    ran: FxHashSet<BindingId>,
    named: FxHashSet<(ModuleId, BindingId)>,
}

impl CompileTimeRoots {
    pub fn new(ran: FxHashSet<BindingId>, named: FxHashSet<(ModuleId, BindingId)>) -> Self {
        Self { ran, named }
    }

    pub fn ran(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.ran.iter().copied()
    }

    pub fn named_by_macros_in(&self, module: ModuleId) -> impl Iterator<Item = BindingId> + '_ {
        self.named
            .iter()
            .filter(move |(named_in, _)| *named_in == module)
            .map(|(_, binding)| *binding)
    }

    pub fn contains(&self, binding: BindingId) -> bool {
        self.ran.contains(&binding) || self.named.iter().any(|(_, named)| *named == binding)
    }
}
