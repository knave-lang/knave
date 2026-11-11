use super::{Scope, ScopeKind};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct ScopeKinds {
    in_order_made: Vec<ScopeKind>,
}

impl ScopeKinds {
    pub fn fresh(&mut self, kind: ScopeKind) -> Scope {
        let scope = self.next_scope();
        self.in_order_made.push(kind);
        scope
    }

    pub fn next_scope(&self) -> Scope {
        let made = u32::try_from(self.in_order_made.len())
            .expect("a program makes fewer than 2^32 scopes");
        Scope(made)
    }

    pub fn kind(&self, scope: Scope) -> Option<ScopeKind> {
        let position = usize::try_from(scope.0).expect("a scope number fits in usize");
        self.in_order_made.get(position).copied()
    }
}
