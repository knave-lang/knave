use crate::scope::{Scope, ScopeSet};

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ModuleScopes {
    base: Scope,
    body: Scope,
}

impl ModuleScopes {
    pub fn new(base: Scope, body: Scope) -> Self {
        Self { base, body }
    }

    pub fn base(self) -> Scope {
        self.base
    }

    pub fn body(self) -> Scope {
        self.body
    }

    pub fn set(self) -> ScopeSet {
        let mut scopes = ScopeSet::default();
        scopes.add(self.base);
        scopes.add(self.body);
        scopes
    }
}
