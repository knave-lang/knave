use rustc_hash::FxHashSet;

use crate::core::BindingId;

pub struct CompileTimeRoots {
    ran: FxHashSet<BindingId>,
    named: FxHashSet<BindingId>,
}

impl CompileTimeRoots {
    pub fn new(ran: FxHashSet<BindingId>, named: FxHashSet<BindingId>) -> Self {
        Self { ran, named }
    }

    pub fn ran(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.ran.iter().copied()
    }

    pub fn is_named_by_a_macro(&self, binding: BindingId) -> bool {
        self.named.contains(&binding)
    }

    pub fn contains(&self, binding: BindingId) -> bool {
        self.ran.contains(&binding) || self.is_named_by_a_macro(binding)
    }
}
