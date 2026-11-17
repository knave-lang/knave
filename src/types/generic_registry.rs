use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::declared_generics::DeclaredGenerics;
use super::ty::{Bound, GenericId, GenericParam};
use super::unifier::LiteralKind;

#[derive(Default)]
pub(super) struct GenericRegistry {
    params: FxHashMap<GenericId, GenericParam>,
    associated_types: FxHashMap<GenericId, Rc<str>>,
    literals: FxHashMap<GenericId, LiteralKind>,
}

impl GenericRegistry {
    pub(super) fn declare(&mut self, name: &str, bounds: Vec<Bound>) -> GenericParam {
        let param = GenericParam {
            id: GenericId::new(self.params.len()),
            name: name.into(),
            bounds,
        };
        self.replace(&param);
        param
    }

    pub(super) fn replace(&mut self, param: &GenericParam) {
        self.params.insert(param.id, param.clone());
    }

    pub(super) fn mark_associated_type(&mut self, generic: GenericId, name: Rc<str>) {
        self.associated_types.insert(generic, name);
    }

    pub(super) fn mark_literal(&mut self, generic: GenericId, literal: LiteralKind) {
        self.literals.insert(generic, literal);
    }

    pub(super) fn param(&self, generic: GenericId) -> Option<&GenericParam> {
        self.params.get(&generic)
    }

    pub(super) fn params(&self) -> &FxHashMap<GenericId, GenericParam> {
        &self.params
    }

    pub(super) fn literal(&self, generic: GenericId) -> Option<LiteralKind> {
        self.literals.get(&generic).copied()
    }
}

impl From<GenericRegistry> for DeclaredGenerics {
    fn from(registry: GenericRegistry) -> Self {
        Self {
            params: registry.params,
            associated_types: registry.associated_types,
            literals: registry.literals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GenericRegistry;
    use crate::core::BindingId;
    use crate::types::declared_generics::DeclaredGenerics;
    use crate::types::ty::{Bound, GenericId};
    use crate::types::unifier::LiteralKind;

    struct RegistryFixture {
        registry: GenericRegistry,
    }

    impl RegistryFixture {
        const DISPLAY: BindingId = BindingId::from_index(0);

        fn new() -> Self {
            Self {
                registry: GenericRegistry::default(),
            }
        }

        fn declared(&mut self, name: &str) -> GenericId {
            self.registry.declare(name, Vec::new()).id
        }
    }

    #[test]
    fn each_declared_generic_gets_the_next_id() {
        let mut fixture = RegistryFixture::new();
        assert_eq!(fixture.declared("a"), GenericId::new(0));
        assert_eq!(fixture.declared("b"), GenericId::new(1));
    }

    #[test]
    fn a_declared_generic_is_found_by_its_id() {
        let mut fixture = RegistryFixture::new();
        let generic = fixture.declared("a");
        let param = fixture.registry.param(generic).expect("a is declared");
        assert_eq!(&*param.name, "a");
        assert!(fixture.registry.params().contains_key(&generic));
    }

    #[test]
    fn a_replaced_generic_keeps_its_new_bounds() {
        let mut fixture = RegistryFixture::new();
        let mut param = fixture.registry.declare("a", Vec::new());
        param.bounds = vec![Bound::plain(RegistryFixture::DISPLAY)];
        fixture.registry.replace(&param);
        let replaced = fixture.registry.param(param.id).expect("a is declared");
        assert_eq!(replaced.bounds, param.bounds);
    }

    #[test]
    fn a_generic_marked_as_a_literal_reads_back_its_literal() {
        let mut fixture = RegistryFixture::new();
        let generic = fixture.declared("a");
        fixture.registry.mark_literal(generic, LiteralKind::Float);
        assert_eq!(fixture.registry.literal(generic), Some(LiteralKind::Float));
    }

    #[test]
    fn an_unmarked_generic_is_no_literal() {
        let mut fixture = RegistryFixture::new();
        let generic = fixture.declared("a");
        assert_eq!(fixture.registry.literal(generic), None);
    }

    #[test]
    fn the_declared_generics_name_each_associated_type() {
        let mut fixture = RegistryFixture::new();
        let generic = fixture.declared("item");
        fixture
            .registry
            .mark_associated_type(generic, "item".into());
        fixture.registry.mark_literal(generic, LiteralKind::Integer);
        let declared = DeclaredGenerics::from(fixture.registry);
        assert_eq!(
            declared.associated_types.get(&generic).map(|name| &**name),
            Some("item")
        );
        assert_eq!(declared.literals.get(&generic), Some(&LiteralKind::Integer));
        assert!(declared.params.contains_key(&generic));
    }
}
