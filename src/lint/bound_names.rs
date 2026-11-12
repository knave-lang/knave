use std::cell::Cell;

use rustc_hash::FxHashMap;

use crate::core::BindingId;
use crate::reader::Symbol;
use crate::scope::Scope;

pub struct BoundNames {
    bindings: FxHashMap<Scope, BindingId>,
}

impl BoundNames {
    pub fn new(bindings: FxHashMap<Scope, BindingId>) -> Self {
        Self { bindings }
    }

    pub fn binding_of(&self, symbol: &Symbol) -> Option<BindingId> {
        let found = Cell::new(None);
        symbol.scopes().any(|scope| {
            found.set(self.bindings.get(&scope).copied());
            found.get().is_some()
        });
        found.get()
    }
}
