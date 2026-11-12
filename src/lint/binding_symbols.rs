use rustc_hash::FxHashMap;

use super::bound_names::BoundNames;
use crate::core::{BindingArena, BindingId};
use crate::reader::Symbol;
use crate::scope::{Scope, ScopeSet, ScopeSupply};

pub struct BindingSymbols<'s> {
    supply: &'s mut ScopeSupply,
    identities: FxHashMap<BindingId, ScopeSet>,
    bindings: FxHashMap<Scope, BindingId>,
}

impl<'s> BindingSymbols<'s> {
    pub fn new(supply: &'s mut ScopeSupply) -> Self {
        Self {
            supply,
            identities: FxHashMap::default(),
            bindings: FxHashMap::default(),
        }
    }

    pub fn symbol(&mut self, binding: BindingId, arena: &BindingArena) -> Symbol {
        let identity = self.identity(binding);
        Symbol::with_scopes(arena.get(binding).shared_name(), identity)
    }

    pub fn into_names(self) -> BoundNames {
        BoundNames::new(self.bindings)
    }

    fn identity(&mut self, binding: BindingId) -> ScopeSet {
        if let Some(identity) = self.identities.get(&binding) {
            return identity.clone();
        }
        let scope = self.supply.fresh();
        let mut identity = ScopeSet::default();
        identity.add(scope);
        self.identities.insert(binding, identity.clone());
        self.bindings.insert(scope, binding);
        identity
    }
}

#[cfg(test)]
mod tests {
    use super::BindingSymbols;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind};
    use crate::lint::expanded_source::ExpandedSource;
    use crate::reader::Symbol;

    struct SymbolsFixture {
        source: ExpandedSource,
        arena: BindingArena,
    }

    impl SymbolsFixture {
        fn new() -> Self {
            Self {
                source: ExpandedSource::expand("(def one 1)"),
                arena: BindingArena::default(),
            }
        }

        fn local(&mut self, name: &str) -> BindingId {
            self.arena
                .add(BindingInfo::new(name.into(), BindingKind::Local, None))
        }

        fn symbols_of(&mut self, bindings: &[BindingId]) -> (Vec<Symbol>, BindingSymbols<'_>) {
            let mut symbols = BindingSymbols::new(self.source.supply());
            let made = bindings
                .iter()
                .map(|binding| symbols.symbol(*binding, &self.arena))
                .collect();
            (made, symbols)
        }
    }

    #[test]
    fn a_symbol_keeps_the_name_of_its_binding() {
        let mut fixture = SymbolsFixture::new();
        let total = fixture.local("total");
        let (made, _) = fixture.symbols_of(&[total]);
        assert_eq!(made[0].name(), "total");
    }

    #[test]
    fn every_symbol_of_one_binding_refers_to_it() {
        let mut fixture = SymbolsFixture::new();
        let total = fixture.local("total");
        let (made, symbols) = fixture.symbols_of(&[total, total]);
        let names = symbols.into_names();
        assert_eq!(names.binding_of(&made[0]), Some(total));
        assert_eq!(names.binding_of(&made[1]), Some(total));
    }

    #[test]
    fn one_binding_gets_one_scope() {
        let mut fixture = SymbolsFixture::new();
        let total = fixture.local("total");
        let (made, _) = fixture.symbols_of(&[total, total]);
        assert_eq!(made[0], made[1]);
    }

    #[test]
    fn two_bindings_of_one_name_stay_apart() {
        let mut fixture = SymbolsFixture::new();
        let outer = fixture.local("total");
        let inner = fixture.local("total");
        let (made, symbols) = fixture.symbols_of(&[inner, outer]);
        assert_eq!(symbols.into_names().binding_of(&made[0]), Some(inner));
    }

    #[test]
    fn a_symbol_the_program_never_bound_refers_to_nothing() {
        let mut fixture = SymbolsFixture::new();
        let (_, symbols) = fixture.symbols_of(&[]);
        assert_eq!(symbols.into_names().binding_of(&Symbol::new("local")), None);
    }
}
