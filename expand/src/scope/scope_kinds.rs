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

#[cfg(test)]
mod tests {
    use super::{ScopeKind, ScopeKinds};
    use crate::scope::ScopeAllocator;

    #[test]
    fn each_fresh_scope_is_new() {
        let mut kinds = ScopeKinds::default();
        let first = kinds.fresh(ScopeKind::Local);
        assert_ne!(kinds.fresh(ScopeKind::Local), first);
    }

    #[test]
    fn the_next_scope_is_the_one_made_next() {
        let mut kinds = ScopeKinds::default();
        kinds.fresh(ScopeKind::Module);
        let next = kinds.next_scope();
        assert_eq!(kinds.fresh(ScopeKind::Local), next);
    }

    #[test]
    fn the_next_scope_comes_after_the_scopes_made() {
        let mut kinds = ScopeKinds::default();
        let made = kinds.fresh(ScopeKind::Module);
        assert_ne!(kinds.next_scope(), made);
    }

    #[test]
    fn a_scope_has_the_kind_it_was_made_with() {
        let mut kinds = ScopeKinds::default();
        kinds.fresh(ScopeKind::Module);
        let use_site = kinds.fresh(ScopeKind::UseSite);
        assert_eq!(kinds.kind(use_site), Some(ScopeKind::UseSite));
    }

    #[test]
    fn a_scope_made_elsewhere_has_no_kind() {
        let kinds = ScopeKinds::default();
        let elsewhere = ScopeAllocator::default().fresh();
        assert_eq!(kinds.kind(elsewhere), None);
    }
}
