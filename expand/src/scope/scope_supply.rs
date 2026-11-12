use super::Scope;

pub struct ScopeSupply {
    next: usize,
}

impl ScopeSupply {
    pub(super) fn starting_at(first: Scope) -> Self {
        Self {
            next: first.position(),
        }
    }

    pub fn fresh(&mut self) -> Scope {
        let scope = Scope::numbered(self.next);
        self.next += 1;
        scope
    }
}

#[cfg(test)]
mod tests {
    use crate::scope::{Scope, ScopeKind, Scopes};

    #[derive(Default)]
    struct SupplyFixture {
        scopes: Scopes,
    }

    impl SupplyFixture {
        const EXPANSION_SCOPES: usize = 3;

        fn expansion_scopes(&mut self) -> Vec<Scope> {
            (0..Self::EXPANSION_SCOPES)
                .map(|_| self.scopes.fresh(ScopeKind::Local))
                .collect()
        }
    }

    #[test]
    fn a_supplied_scope_is_none_that_expansion_made() {
        let mut fixture = SupplyFixture::default();
        let made = fixture.expansion_scopes();
        let supplied = fixture.scopes.into_supply().fresh();
        assert!(!made.contains(&supplied));
    }

    #[test]
    fn two_supplied_scopes_differ() {
        let mut fixture = SupplyFixture::default();
        fixture.expansion_scopes();
        let mut supply = fixture.scopes.into_supply();
        assert_ne!(supply.fresh(), supply.fresh());
    }
}
