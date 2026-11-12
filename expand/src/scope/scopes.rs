use super::{Scope, ScopeKind, ScopeSupply};
use crate::reader::Form;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Scopes {
    kinds_in_order_made: Vec<ScopeKind>,
}

impl Scopes {
    pub(crate) fn fresh(&mut self, kind: ScopeKind) -> Scope {
        let scope = self.next_scope();
        self.kinds_in_order_made.push(kind);
        scope
    }

    pub(crate) fn next_scope(&self) -> Scope {
        Scope::numbered(self.kinds_in_order_made.len())
    }

    pub(crate) fn into_supply(self) -> ScopeSupply {
        ScopeSupply::starting_at(self.next_scope())
    }

    pub(crate) fn quoted(&self, datum: &Form) -> Form {
        let mut quoted = datum.clone();
        quoted.retain_scopes(&|scope| !self.is_stripped_by_quote(scope));
        quoted
    }

    pub(crate) fn carries_introduction_scope(&self, binder: &Form) -> bool {
        binder.as_symbol().is_some_and(|symbol| {
            symbol
                .scopes()
                .any(|scope| self.kind(scope).is_some_and(ScopeKind::is_introduction))
        })
    }

    fn is_stripped_by_quote(&self, scope: Scope) -> bool {
        self.kind(scope)
            .is_some_and(ScopeKind::is_stripped_by_quote)
    }

    fn kind(&self, scope: Scope) -> Option<ScopeKind> {
        self.kinds_in_order_made.get(scope.position()).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::Scopes;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::scope::{Scope, ScopeKind, ScopeSet};
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
    fn a_quote_keeps_a_scope_with_no_recorded_kind() {
        let fixture = ScopesFixture::default();
        let unrecorded = Scope::numbered(0);
        let symbol = ScopesFixture::symbol_with_scope(unrecorded);
        assert!(fixture.keeps_scope_through_a_quote(&symbol, unrecorded));
    }

    #[test]
    fn a_scope_keeps_its_own_kind_beside_earlier_scopes() {
        let mut fixture = ScopesFixture::default();
        fixture.scopes.fresh(ScopeKind::Module);
        assert!(!fixture.keeps_through_a_quote(ScopeKind::Local));
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

    #[test]
    fn each_fresh_scope_is_new() {
        let mut scopes = Scopes::default();
        let first = scopes.fresh(ScopeKind::Local);
        assert_ne!(scopes.fresh(ScopeKind::Local), first);
    }

    #[test]
    fn the_next_scope_is_the_one_made_next() {
        let mut scopes = Scopes::default();
        scopes.fresh(ScopeKind::Module);
        let next = scopes.next_scope();
        assert_eq!(scopes.fresh(ScopeKind::Local), next);
    }

    #[test]
    fn the_next_scope_comes_after_the_scopes_made() {
        let mut scopes = Scopes::default();
        let made = scopes.fresh(ScopeKind::Module);
        assert_ne!(scopes.next_scope(), made);
    }
}
