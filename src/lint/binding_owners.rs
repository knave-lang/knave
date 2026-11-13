use rustc_hash::FxHashMap;

use crate::core::{BindingArena, BindingId, BindingKind, MemberOwners};
use crate::expand::Program;

pub struct BindingOwners {
    owners: FxHashMap<BindingId, BindingId>,
}

impl BindingOwners {
    pub fn of(program: &Program) -> Self {
        let mut owners = FxHashMap::default();
        for item in program.modules.iter().flat_map(|module| &module.items) {
            MemberOwners::of(&item.kind).visit(&mut |member, owner| {
                owners.insert(member, owner);
            });
        }
        Self { owners }
    }

    pub fn owner(&self, binding: BindingId, bindings: &BindingArena) -> BindingId {
        match bindings.get(binding).kind() {
            BindingKind::Variant(_, info) => info.owner,
            _ => self.owners.get(&binding).copied().unwrap_or(binding),
        }
    }
}
