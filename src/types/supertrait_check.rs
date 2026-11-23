use super::declarations::Declarations;
use super::display::TypeDisplay;
use super::problems::TypeProblems;
use super::ty::{Bound, Type};
use super::typed_trait_impl::TypedTraitImpl;
use super::unifier::Unifier;
use crate::core::{BindingArena, BindingId};
use crate::diagnostic::{Diagnostic, Diagnostics};

pub(super) struct SupertraitCheck<'c> {
    declarations: &'c Declarations,
    bindings: &'c BindingArena,
    unifier: &'c Unifier,
}

impl<'c> SupertraitCheck<'c> {
    pub(super) fn new(
        declarations: &'c Declarations,
        bindings: &'c BindingArena,
        unifier: &'c Unifier,
    ) -> Self {
        Self {
            declarations,
            bindings,
            unifier,
        }
    }

    pub(super) fn missing(&self, display: &TypeDisplay) -> Diagnostics {
        let implementations = self.declarations.traits().impls.iter();
        implementations
            .flat_map(|implementation| {
                let implementor = &implementation.implementor;
                let shown = display.show(implementor);
                (self.supertraits(implementation.trait_binding).into_iter())
                    .filter(|supertrait| !self.is_met(supertrait, implementor))
                    .map(move |supertrait| self.missing_one(implementation, &supertrait, &shown))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn supertraits(&self, trait_binding: BindingId) -> Vec<Bound> {
        let table = &self.declarations.traits().table;
        (table.get(trait_binding))
            .map(|rust_trait| rust_trait.rows().reached().supertraits().to_vec())
            .unwrap_or_default()
    }

    fn is_met(&self, supertrait: &Bound, implementor: &Type) -> bool {
        self.declarations
            .traits()
            .table
            .satisfies(supertrait, implementor, &|generic, bound| {
                self.unifier.generic_bounds(generic).contains(bound)
            })
    }

    fn missing_one(
        &self,
        implementation: &TypedTraitImpl,
        supertrait: &Bound,
        implementor: &str,
    ) -> Diagnostic {
        let table = &self.declarations.traits().table;
        let help = (table.get(supertrait.binding)).and_then(|known| known.help.clone());
        TypeProblems::missing_supertrait(
            self.bindings.get(implementation.trait_binding).name(),
            self.bindings.get(supertrait.binding).name(),
            implementor,
            implementation.implementor_span,
            help,
        )
    }
}
