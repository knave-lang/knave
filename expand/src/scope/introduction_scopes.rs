use super::{Scope, ScopeKind, ScopeSet};
use crate::reader::{Form, Symbol};

pub(crate) struct IntroductionScopes<'s> {
    kinds_in_order_made: &'s [ScopeKind],
}

impl<'s> IntroductionScopes<'s> {
    pub(super) fn over(kinds_in_order_made: &'s [ScopeKind]) -> Self {
        Self {
            kinds_in_order_made,
        }
    }

    pub(crate) fn carried_by(&self, symbol: &Symbol) -> bool {
        symbol.scopes().any(|scope| self.is_introduction(scope))
    }

    pub(crate) fn carried_by_binder(&self, binder: &Form) -> bool {
        binder
            .as_symbol()
            .is_some_and(|symbol| self.carried_by(symbol))
    }

    pub(crate) fn latest_at_head(&self, form: &Form) -> Option<Scope> {
        let head = form.as_list()?.first()?.as_symbol()?;
        head.scopes()
            .largest_matching(|scope| self.is_introduction(scope))
    }

    pub(crate) fn equal_without_introductions(
        &self,
        written: &ScopeSet,
        stored: &ScopeSet,
    ) -> bool {
        stored.is_subset_of(written)
            && !written.any(|scope| !self.is_introduction(scope) && !stored.contains(scope))
    }

    pub(crate) fn remove_from(&self, scopes: &mut ScopeSet) {
        scopes.retain(|scope| !self.is_introduction(scope));
    }

    fn is_introduction(&self, scope: Scope) -> bool {
        self.kinds_in_order_made
            .get(scope.position())
            .is_some_and(|kind| kind.is_introduction())
    }
}
