use rustc_hash::FxHashMap;

use super::declarations::Declarations;
use super::generic_declarations::GenericDeclarations;
use super::impl_absorption::ImplAbsorption;
use super::impl_bound_seeds::ImplBoundSeeds;
use super::impl_position::ImplPosition;
use super::implied_bounds::ImpliedBounds;
use super::ty::{Bound, GenericId, Type};
use super::typed::TypedDefinition;
use crate::core::BindingId;

pub(super) struct ImplBounds<'a> {
    declarations: &'a mut Declarations,
    implied: &'a ImpliedBounds,
    generics: GenericDeclarations<'a>,
}

impl<'a> ImplBounds<'a> {
    pub(super) fn new(
        declarations: &'a mut Declarations,
        implied: &'a ImpliedBounds,
        generics: GenericDeclarations<'a>,
    ) -> Self {
        Self {
            declarations,
            implied,
            generics,
        }
    }

    pub(super) fn seed(mut self, seeds: &ImplBoundSeeds) {
        for position in ImplPosition::all(self.declarations.traits().impls.len()) {
            let params = &self.declarations.traits().impls[position.index()].params;
            if let Some(bounds) = seeds.bounds_of(position, params) {
                self.store(position, bounds);
            }
        }
    }

    pub(super) fn absorb(
        mut self,
        positions: &[ImplPosition],
        definitions: &FxHashMap<BindingId, TypedDefinition>,
        absorption: &mut ImplAbsorption,
    ) {
        for &position in positions {
            let found = self.found_bounds(position, definitions);
            let params = &self.declarations.traits().impls[position.index()].params;
            absorption.record(position, params, &found);
            self.store(position, found);
        }
    }

    fn found_bounds(
        &self,
        position: ImplPosition,
        definitions: &FxHashMap<BindingId, TypedDefinition>,
    ) -> Vec<Vec<Bound>> {
        let implementation = &self.declarations.traits().impls[position.index()];
        let params = &implementation.params;
        let mut found: Vec<Vec<Bound>> = vec![Vec::new(); params.len()];
        for method in &implementation.methods {
            let Some(definition) = definitions.get(&method.binding) else {
                continue;
            };
            let replacements: FxHashMap<GenericId, Type> = definition
                .scheme
                .params
                .iter()
                .zip(params)
                .map(|(method_param, implementation)| {
                    (method_param.id, Type::Generic(implementation.id))
                })
                .collect();
            for (bounds, method_param) in found.iter_mut().zip(&definition.scheme.params) {
                let declared = method_param.bounds.iter();
                let implied = self.implied.of(method_param.id);
                for bound in declared.chain(implied) {
                    let bound = bound.substituted(&replacements);
                    if !bounds.contains(&bound) {
                        bounds.push(bound);
                    }
                }
            }
        }
        found
    }

    fn store(&mut self, position: ImplPosition, bounds: Vec<Vec<Bound>>) {
        let mut params = self.declarations.traits().impls[position.index()]
            .params
            .clone();
        for (param, bounds) in params.iter_mut().zip(bounds) {
            self.generics.redeclare(param, bounds);
        }
        self.declarations
            .record_impl_params(position.index(), params);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::ty::GenericParam;

    const SOURCE: &str = "
        (trait same-ends (same-ends (fn ((ref self)) bool)))
        (impl same-ends (vec a) (same-ends (items) (= (at items 0) (at items 1))))
        (defn main () (do))";

    struct AbsorbedFixture {
        declarations: DeclarationsFixture,
    }

    impl AbsorbedFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn impl_param(&self) -> GenericParam {
            let same_ends = self.declarations.trait_named("same-ends");
            let implementation = self
                .declarations
                .traits()
                .impls()
                .iter()
                .find(|implementation| implementation.trait_binding == same_ends)
                .expect("the source implements same-ends");
            implementation.params[0].clone()
        }
    }

    #[test]
    fn an_absorbed_bound_names_the_impl_parameter() {
        let param = AbsorbedFixture::new().impl_param();
        let argument = param.bounds[0].argument.clone();
        assert_eq!(argument, Some(Type::Generic(param.id)));
    }
}
