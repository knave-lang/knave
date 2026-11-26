use rustc_hash::FxHashMap;

use super::rust_trait_paths::RustTraitPaths;
use super::structural_trait::StructuralTrait;
use super::trait_methods::TraitMethods;
use super::traits::TraitTable;
use super::ty::{GenericParam, Scheme};
use super::typed_trait::TypedTrait;
use super::typed_trait_impl::TypedTraitImpl;
use crate::core::BindingId;

#[derive(Default)]
pub struct TraitDeclarations {
    pub(super) table: TraitTable,
    pub(super) definitions: Vec<TypedTrait>,
    pub(super) impls: Vec<TypedTraitImpl>,
    pub(super) methods: TraitMethods,
    pub(super) impl_methods: FxHashMap<BindingId, usize>,
    pub(super) rust_paths: RustTraitPaths,
    pub(super) structural: Vec<StructuralTrait>,
}

impl TraitDeclarations {
    pub(super) fn record_impl_params(&mut self, position: usize, params: Vec<GenericParam>) {
        let implementation = &mut self.impls[position];
        implementation.params = params.clone();
        let implementor = implementation.implementor.clone();
        if let Some(rows) = self.table.reached_rows_mut(implementation.trait_binding) {
            let scheme = Scheme {
                lifetimes: Vec::new(),
                params,
                body: implementor.clone(),
            };
            rows.set_scheme(&implementor, scheme);
        }
    }

    pub fn table(&self) -> &TraitTable {
        &self.table
    }

    pub fn definitions(&self) -> &[TypedTrait] {
        &self.definitions
    }

    pub fn impls(&self) -> &[TypedTraitImpl] {
        &self.impls
    }

    pub fn methods(&self) -> &TraitMethods {
        &self.methods
    }

    pub fn rust_paths(&self) -> &RustTraitPaths {
        &self.rust_paths
    }

    pub fn structural(&self) -> &[StructuralTrait] {
        &self.structural
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::ty::Bound;

    const SOURCE: &str = "
        (trait shown (show (fn ((ref self)) int)))
        (impl shown (vec a) (show (items) (len items)))
        (defn main () (do))";

    struct ImplParamsFixture {
        declarations: DeclarationsFixture,
    }

    impl ImplParamsFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn shown_position(&self) -> usize {
            let shown = self.declarations.trait_named("shown");
            self.declarations
                .traits()
                .impls()
                .iter()
                .position(|implementation| implementation.trait_binding == shown)
                .expect("the source implements shown")
        }

        fn require_clone(&mut self) {
            let position = self.shown_position();
            let mut params = self.declarations.traits().impls()[position].params.clone();
            let clone = self.declarations.trait_named("clone");
            params[0].bounds.push(Bound::plain(clone));
            self.declarations.record_impl_params(position, params);
        }

        fn shows_errors(&self) -> bool {
            let errors = Type::Named(
                self.declarations.rust_type_named("vec"),
                vec![Type::Named(
                    self.declarations.rust_type_named("io-error"),
                    Vec::new(),
                )],
            );
            let shown = Bound::plain(self.declarations.trait_named("shown"));
            self.declarations
                .traits()
                .table()
                .satisfies(&shown, &errors, &|_, _| false)
        }
    }

    #[test]
    fn recorded_params_replace_the_impl_params() {
        let mut fixture = ImplParamsFixture::new();
        fixture.require_clone();
        let position = fixture.shown_position();
        let params = &fixture.declarations.traits().impls()[position].params;
        assert_eq!(params[0].bounds.len(), 1);
    }

    #[test]
    fn recorded_params_bound_the_implementor_in_the_trait_table() {
        let mut fixture = ImplParamsFixture::new();
        assert!(fixture.shows_errors());
        fixture.require_clone();
        assert!(!fixture.shows_errors());
    }
}
