use crate::reader::Form;
use crate::scope::{Scope, ScopeKind, ScopeKinds};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Scopes {
    kinds: ScopeKinds,
}

impl Scopes {
    pub(super) fn fresh(&mut self, kind: ScopeKind) -> Scope {
        self.kinds.fresh(kind)
    }

    pub(super) fn next_scope(&self) -> Scope {
        self.kinds.next_scope()
    }

    pub(super) fn quoted(&self, datum: &Form) -> Form {
        let mut quoted = datum.clone();
        quoted.retain_scopes(&|scope| !self.is_stripped_by_quote(scope));
        quoted
    }

    pub(super) fn carries_introduction_scope(&self, binder: &Form) -> bool {
        binder.as_symbol().is_some_and(|symbol| {
            symbol
                .scopes()
                .any(|scope| self.kinds.kind(scope) == Some(ScopeKind::Introduction))
        })
    }

    fn is_stripped_by_quote(&self, scope: Scope) -> bool {
        self.kinds
            .kind(scope)
            .is_some_and(ScopeKind::is_stripped_by_quote)
    }
}
