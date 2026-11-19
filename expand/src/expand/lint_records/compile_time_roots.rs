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

    pub fn contains(&self, binding: BindingId) -> bool {
        self.ran.contains(&binding) || self.named.contains(&binding)
    }
}
