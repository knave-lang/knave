use std::rc::Rc;

use super::generic_registry::GenericRegistry;
use super::ty::{Bound, GenericParam, Type, TypeVar};
use super::unifier::Unifier;

pub(super) struct GenericDeclarations<'a> {
    registry: &'a mut GenericRegistry,
    unifier: &'a mut Unifier,
}

impl<'a> GenericDeclarations<'a> {
    pub(super) fn new(registry: &'a mut GenericRegistry, unifier: &'a mut Unifier) -> Self {
        Self { registry, unifier }
    }

    pub(super) fn declare(&mut self, name: &str, bounds: Vec<Bound>) -> GenericParam {
        let param = self.registry.declare(name, bounds.clone());
        self.unifier.register_generic(param.id, bounds);
        param
    }

    pub(super) fn redeclare(&mut self, param: &mut GenericParam, bounds: Vec<Bound>) {
        param.bounds = bounds.clone();
        self.registry.replace(param);
        self.unifier.register_generic(param.id, bounds);
    }

    pub(super) fn generalize(
        &mut self,
        var: TypeVar,
        name: &str,
        bounds: Vec<Bound>,
    ) -> GenericParam {
        let param = self.declare(name, bounds);
        self.unifier.bind(var, Type::Generic(param.id));
        param
    }

    pub(super) fn declare_associated_type(&mut self, name: &str) -> GenericParam {
        let param = self.declare(name, Vec::new());
        self.registry.mark_associated_type(param.id, Rc::from(name));
        param
    }

    pub(super) fn fresh_variable(&mut self) -> Type {
        self.unifier.fresh()
    }
}

#[cfg(test)]
mod tests {
    use super::GenericDeclarations;
    use crate::core::BindingId;
    use crate::types::declared_generics::DeclaredGenerics;
    use crate::types::generic_registry::GenericRegistry;
    use crate::types::ty::{Bound, Type};
    use crate::types::unifier::Unifier;

    struct DeclarationsFixture {
        registry: GenericRegistry,
        unifier: Unifier,
    }

    impl DeclarationsFixture {
        const DISPLAY: BindingId = BindingId::from_index(0);
        const DEBUG: BindingId = BindingId::from_index(1);

        fn new() -> Self {
            Self {
                registry: GenericRegistry::default(),
                unifier: Unifier::default(),
            }
        }

        fn declarations(&mut self) -> GenericDeclarations<'_> {
            GenericDeclarations::new(&mut self.registry, &mut self.unifier)
        }
    }

    #[test]
    fn a_declared_generic_registers_its_bounds_with_the_unifier() {
        let mut fixture = DeclarationsFixture::new();
        let display = vec![Bound::plain(DeclarationsFixture::DISPLAY)];
        let param = fixture.declarations().declare("a", display.clone());
        assert_eq!(fixture.unifier.generic_bounds(param.id), display.as_slice());
        assert!(fixture.registry.param(param.id).is_some());
    }

    #[test]
    fn a_redeclared_generic_replaces_its_bounds_in_the_unifier_and_the_registry() {
        let mut fixture = DeclarationsFixture::new();
        let mut param = fixture.declarations().declare("a", Vec::new());
        let debug = vec![Bound::plain(DeclarationsFixture::DEBUG)];
        fixture.declarations().redeclare(&mut param, debug);
        assert_eq!(
            fixture.unifier.generic_bounds(param.id),
            param.bounds.as_slice()
        );
        let registered = fixture.registry.param(param.id).expect("a is declared");
        assert_eq!(registered.bounds, param.bounds);
    }

    #[test]
    fn a_generalized_variable_resolves_to_its_generic() {
        let mut fixture = DeclarationsFixture::new();
        let Type::Var(var) = fixture.unifier.fresh() else {
            unreachable!("fresh returns a variable")
        };
        let param = fixture.declarations().generalize(var, "a", Vec::new());
        assert_eq!(
            fixture.unifier.resolve(&Type::Var(var)),
            Type::Generic(param.id)
        );
    }

    #[test]
    fn a_declared_associated_type_is_named_in_the_registry() {
        let mut fixture = DeclarationsFixture::new();
        let param = fixture.declarations().declare_associated_type("item");
        let declared = DeclaredGenerics::from(fixture.registry);
        assert_eq!(
            declared.associated_types.get(&param.id).map(|name| &**name),
            Some("item")
        );
        assert!(declared.params.contains_key(&param.id));
    }

    #[test]
    fn each_fresh_variable_is_a_new_variable() {
        let mut fixture = DeclarationsFixture::new();
        let first = fixture.declarations().fresh_variable();
        let second = fixture.declarations().fresh_variable();
        assert!(matches!(first, Type::Var(_)));
        assert_ne!(first, second);
    }
}
