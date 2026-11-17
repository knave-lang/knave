use rustc_hash::FxHashMap;

use super::generic_registry::GenericRegistry;
use super::implied_bounds::ImpliedBounds;
use super::ty::{GenericId, Scheme, Type};
use super::unifier::Unifier;
use crate::source::Span;

pub(super) struct Instantiation<'a> {
    generics: &'a GenericRegistry,
    implied: &'a ImpliedBounds,
    unifier: &'a mut Unifier,
}

impl<'a> Instantiation<'a> {
    pub(super) fn new(
        generics: &'a GenericRegistry,
        implied: &'a ImpliedBounds,
        unifier: &'a mut Unifier,
    ) -> Self {
        Self {
            generics,
            implied,
            unifier,
        }
    }

    pub(super) fn instantiate(&mut self, scheme: &Scheme, span: Span) -> Type {
        let fresh: Vec<Type> = scheme
            .params
            .iter()
            .map(|param| self.fresh_instance(param.id))
            .collect();
        let replacements: FxHashMap<GenericId, Type> = scheme
            .params
            .iter()
            .zip(&fresh)
            .map(|(param, value)| (param.id, value.clone()))
            .collect();
        for (param, value) in scheme.params.iter().zip(&fresh) {
            if let Type::Var(var) = value {
                let bounds = param
                    .bounds
                    .iter()
                    .chain(self.implied.of(param.id))
                    .map(|bound| bound.substituted(&replacements))
                    .collect();
                self.unifier.set_bounds(*var, bounds, span);
            }
        }
        scheme.body.substitute(&replacements)
    }

    fn fresh_instance(&mut self, generic: GenericId) -> Type {
        match self.generics.literal(generic) {
            Some(literal) => self.unifier.fresh_literal(literal),
            None => self.unifier.fresh(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Instantiation;
    use crate::core::BindingId;
    use crate::source::{SourceId, Span};
    use crate::types::generic_registry::GenericRegistry;
    use crate::types::implied_bounds::ImpliedBounds;
    use crate::types::ty::{Bound, GenericParam, Scheme, Type, TypeVar};
    use crate::types::unifier::{LiteralKind, Unifier};

    struct InstantiationFixture {
        registry: GenericRegistry,
        implied: ImpliedBounds,
        unifier: Unifier,
        param: GenericParam,
    }

    impl InstantiationFixture {
        const DISPLAY: BindingId = BindingId::from_index(0);
        const DEBUG: BindingId = BindingId::from_index(1);

        fn new() -> Self {
            let mut registry = GenericRegistry::default();
            let param = registry.declare("a", vec![Bound::plain(Self::DISPLAY)]);
            Self {
                registry,
                implied: ImpliedBounds::default(),
                unifier: Unifier::default(),
                param,
            }
        }

        fn instance(&mut self) -> TypeVar {
            let scheme = Scheme {
                lifetimes: Vec::new(),
                params: vec![self.param.clone()],
                body: Type::Generic(self.param.id),
            };
            let span = Span::new(SourceId::new(0), 0..0);
            let instance = Instantiation::new(&self.registry, &self.implied, &mut self.unifier)
                .instantiate(&scheme, span);
            let Type::Var(var) = instance else {
                unreachable!("the scheme's body is its generic")
            };
            var
        }
    }

    #[test]
    fn an_instance_carries_the_declared_bounds_of_its_generic() {
        let mut fixture = InstantiationFixture::new();
        let var = fixture.instance();
        assert_eq!(
            fixture.unifier.bounds(var),
            [Bound::plain(InstantiationFixture::DISPLAY)]
        );
    }

    #[test]
    fn an_instance_carries_the_implied_bounds_of_its_generic() {
        let mut fixture = InstantiationFixture::new();
        fixture
            .implied
            .record(fixture.param.id, Bound::plain(InstantiationFixture::DEBUG));
        let var = fixture.instance();
        assert!(
            fixture
                .unifier
                .bounds(var)
                .contains(&Bound::plain(InstantiationFixture::DEBUG))
        );
    }

    #[test]
    fn a_literal_generic_instantiates_as_a_literal_variable() {
        let mut fixture = InstantiationFixture::new();
        fixture
            .registry
            .mark_literal(fixture.param.id, LiteralKind::Integer);
        let var = fixture.instance();
        assert_eq!(
            fixture.unifier.literal_kind(var),
            Some(LiteralKind::Integer)
        );
    }

    #[test]
    fn a_plain_generic_instantiates_as_a_plain_variable() {
        let mut fixture = InstantiationFixture::new();
        let var = fixture.instance();
        assert_eq!(fixture.unifier.literal_kind(var), None);
    }

    #[test]
    fn each_instance_gets_a_fresh_variable() {
        let mut fixture = InstantiationFixture::new();
        assert_ne!(fixture.instance(), fixture.instance());
    }
}
