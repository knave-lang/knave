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

#[cfg(test)]
mod tests {
    use super::Scopes;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::scope::{Scope, ScopeAllocator, ScopeKind, ScopeSet};
    use crate::source::{SourceId, Span};

    #[derive(Default)]
    struct ScopesFixture {
        scopes: Scopes,
    }

    impl ScopesFixture {
        fn symbol_with(&mut self, kind: ScopeKind) -> (Form, Scope) {
            let scope = self.scopes.fresh(kind);
            (Self::symbol_with_scope(scope), scope)
        }

        fn symbol_with_scope(scope: Scope) -> Form {
            let mut set = ScopeSet::default();
            set.add(scope);
            Self::form(FormKind::Symbol(Symbol::with_scopes("name", set)))
        }

        fn keeps_through_a_quote(&mut self, kind: ScopeKind) -> bool {
            let (symbol, scope) = self.symbol_with(kind);
            self.keeps_scope_through_a_quote(&symbol, scope)
        }

        fn keeps_scope_through_a_quote(&self, symbol: &Form, scope: Scope) -> bool {
            self.scopes
                .quoted(symbol)
                .as_symbol()
                .is_some_and(|symbol| symbol.scopes().contains(scope))
        }

        fn carries_introduction_scope(&mut self, kind: ScopeKind) -> bool {
            let (symbol, _) = self.symbol_with(kind);
            self.scopes.carries_introduction_scope(&symbol)
        }

        fn form(kind: FormKind) -> Form {
            Form::new(kind, Span::new(SourceId::new(0), 0..0))
        }
    }

    #[test]
    fn a_quote_strips_a_local_scope() {
        assert!(!ScopesFixture::default().keeps_through_a_quote(ScopeKind::Local));
    }

    #[test]
    fn a_quote_strips_a_use_site_scope() {
        assert!(!ScopesFixture::default().keeps_through_a_quote(ScopeKind::UseSite));
    }

    #[test]
    fn a_quote_keeps_a_module_scope() {
        assert!(ScopesFixture::default().keeps_through_a_quote(ScopeKind::Module));
    }

    #[test]
    fn a_quote_keeps_an_introduction_scope() {
        assert!(ScopesFixture::default().keeps_through_a_quote(ScopeKind::Introduction));
    }

    #[test]
    fn a_quote_keeps_a_scope_made_elsewhere() {
        let fixture = ScopesFixture::default();
        let elsewhere = ScopeAllocator::default().fresh();
        let symbol = ScopesFixture::symbol_with_scope(elsewhere);
        assert!(fixture.keeps_scope_through_a_quote(&symbol, elsewhere));
    }

    #[test]
    fn a_symbol_with_an_introduction_scope_carries_one() {
        assert!(ScopesFixture::default().carries_introduction_scope(ScopeKind::Introduction));
    }

    #[test]
    fn a_symbol_with_a_scope_of_another_kind_carries_none() {
        let mut fixture = ScopesFixture::default();
        for kind in [ScopeKind::Module, ScopeKind::Local, ScopeKind::UseSite] {
            assert!(!fixture.carries_introduction_scope(kind), "{kind:?}");
        }
    }

    #[test]
    fn a_list_carries_no_introduction_scope() {
        let mut fixture = ScopesFixture::default();
        let (symbol, _) = fixture.symbol_with(ScopeKind::Introduction);
        let list = ScopesFixture::form(FormKind::List(vec![symbol]));
        assert!(!fixture.scopes.carries_introduction_scope(&list));
    }
}
