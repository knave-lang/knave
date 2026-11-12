use rustc_hash::FxHashSet;

use crate::core::{BindingId, ModuleId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Crate,
    Public,
}

impl Visibility {
    pub fn keyword(self) -> &'static str {
        match self {
            Visibility::Private => "",
            Visibility::Crate => "pub(crate) ",
            Visibility::Public => "pub ",
        }
    }
}

pub struct ItemVisibilities {
    exported: FxHashSet<BindingId>,
    crate_visible: FxHashSet<BindingId>,
}

impl ItemVisibilities {
    pub fn new(exported: FxHashSet<BindingId>) -> Self {
        Self {
            exported,
            crate_visible: FxHashSet::default(),
        }
    }

    pub fn note_use(&mut self, binding: BindingId, owner: ModuleId, from: ModuleId) {
        if owner != from && !self.exported.contains(&binding) {
            self.crate_visible.insert(binding);
        }
    }

    pub fn of(&self, binding: BindingId) -> Visibility {
        if self.exported.contains(&binding) {
            Visibility::Public
        } else if self.crate_visible.contains(&binding) {
            Visibility::Crate
        } else {
            Visibility::Private
        }
    }
}
