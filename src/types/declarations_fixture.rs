use rustc_hash::{FxHashMap, FxHashSet};

use super::collected_impl::CollectedImpl;
use super::declarations::Declarations;
use super::declarations_builder::DeclarationsBuilder;
use super::declared_traits::DeclaredTraits;
use super::declared_types::DeclaredTypes;
use super::derivable_traits::DerivableTraits;
use super::derive_blockers::DeriveBlockers;
use super::derive_catalog::DeriveCatalog;
use super::derive_rules::DeriveRules;
use super::display::TypeDisplay;
use super::explanation_sources::ExplanationSources;
use super::impl_item::ImplItem;
use super::implementation_check::ImplementationCheck;
use super::implementation_collector::ImplementationCollector;
use super::trait_boxing::TraitBoxing;
use super::trait_declarations::TraitDeclarations;
use super::ty::{GenericId, GenericParam};
use super::type_declarations::TypeDeclarations;
use super::variant_payloads::VariantPayloads;
use super::{Type, TypedProgram, TypedSource};
use crate::core::{BindingId, ItemKind};

pub(super) struct DeclarationsFixture {
    source: TypedSource,
    program: TypedProgram,
    declarations: Declarations,
    unchanged_structs: FxHashSet<BindingId>,
}

impl DeclarationsFixture {
    pub(super) fn new(source: &str) -> Self {
        let source = TypedSource::expand(source);
        let mut program = source.typed();
        let declarations = std::mem::take(&mut program.declarations);
        Self {
            source,
            program,
            declarations,
            unchanged_structs: FxHashSet::default(),
        }
    }

    pub(super) fn declarations(&self) -> &Declarations {
        &self.declarations
    }

    pub(super) fn types(&self) -> &TypeDeclarations {
        self.declarations.types()
    }

    pub(super) fn trait_boxing(&self) -> TraitBoxing<'_> {
        TraitBoxing::new(self.declarations.traits(), &self.program.bindings)
    }

    pub(super) fn declared_types(&self) -> DeclaredTypes<'_> {
        DeclaredTypes::new(self.declarations.types())
    }

    pub(super) fn declared_traits(&self) -> DeclaredTraits<'_> {
        DeclaredTraits::new(self.declarations.traits())
    }

    pub(super) fn program_bindings(&self) -> &crate::core::BindingArena {
        &self.program.bindings
    }

    pub(super) fn modules(&self) -> &[crate::core::CoreModule] {
        &self.source.program().modules
    }

    pub(super) fn derive_catalog(&self) -> DeriveCatalog {
        DeriveCatalog::of(self.modules(), &self.declarations.traits().structural)
    }

    pub(super) fn derive_rules<'a>(
        &'a self,
        changed_structs: &'a FxHashSet<BindingId>,
    ) -> DeriveRules<'a> {
        DeriveRules::new(&self.declarations, self.derive_blockers(), changed_structs)
    }

    pub(super) fn explanation_sources(&self) -> ExplanationSources<'_> {
        ExplanationSources {
            declarations: &self.declarations,
            bindings: &self.program.bindings,
            display: TypeDisplay::new(&self.program.bindings, &self.program.generics)
                .with_projections(self.declarations.traits().table.projections()),
            derive_rules: self.derive_rules(&self.unchanged_structs),
        }
    }

    pub(super) fn derivable_traits(&self) -> DerivableTraits<'_> {
        DerivableTraits::new(self.declarations.traits())
    }

    pub(super) fn derive_blockers(&self) -> DeriveBlockers<'_> {
        DeriveBlockers::new(
            &self.declarations,
            VariantPayloads::new(self.declarations.types()),
        )
    }

    pub(super) fn implementation_check(&self) -> ImplementationCheck<'_> {
        let display = TypeDisplay::new(&self.program.bindings, &self.program.generics)
            .with_projections(self.declarations.traits().table.projections());
        ImplementationCheck::new(&self.program.bindings, &self.declarations, display)
    }

    pub(super) fn impl_item(&self, trait_name: &str) -> ImplItem<'_> {
        let trait_binding = self.trait_named(trait_name);
        self.source
            .program()
            .modules
            .iter()
            .flat_map(|module| module.items.iter().map(move |item| (module.id, item)))
            .find_map(|(module, item)| match &item.kind {
                ItemKind::Impl(binding, implementor, associated, methods)
                    if *binding == trait_binding =>
                {
                    Some(ImplItem {
                        module,
                        trait_binding,
                        implementor,
                        associated,
                        methods,
                    })
                }
                _ => None,
            })
            .expect("the source implements the trait")
    }

    pub(super) fn implementation_collector(&self) -> ImplementationCollector<'_> {
        ImplementationCollector::new(self.declared_traits())
    }

    pub(super) fn builder(&mut self) -> DeclarationsBuilder<'_> {
        self.declarations.open_builder()
    }

    pub(super) fn record_impl_params(&mut self, position: usize, params: Vec<GenericParam>) {
        self.declarations.record_impl_params(position, params);
    }

    pub(super) fn enter_impl(&mut self, collected: CollectedImpl) {
        let mut synthetic = Default::default();
        let traits = self.declarations.open_builder().traits();
        traits.enter_impl(collected, &mut synthetic);
    }

    pub(super) fn traits(&self) -> &TraitDeclarations {
        self.declarations.traits()
    }

    pub(super) fn generics(&self) -> &FxHashMap<GenericId, GenericParam> {
        &self.program.generics
    }

    pub(super) fn binding(&self, name: &str) -> BindingId {
        self.binding_named(name, &|_| true)
    }

    pub(super) fn trait_named(&self, name: &str) -> BindingId {
        self.binding_named(name, &|binding| {
            self.declarations.traits().table.get(binding).is_some()
        })
    }

    pub(super) fn rust_type_named(&self, name: &str) -> BindingId {
        let types = self.declarations.types();
        self.binding_named(name, &|binding| types.rust_types.get(binding).is_some())
    }

    pub(super) fn type_named(&self, name: &str) -> BindingId {
        let types = self.declarations.types();
        self.binding_named(name, &|binding| {
            types.structs.get(binding).is_some() || types.enums.get(binding).is_some()
        })
    }

    pub(super) fn parameter(&self, function: &str) -> Type {
        let definition = self
            .program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .find(|definition| self.program.bindings.get(definition.binding).name() == function)
            .expect("the source defines the function");
        let Type::Function(_, parameters, _) = &definition.scheme.body else {
            panic!("a defn has a function type");
        };
        parameters[0].clone()
    }

    fn binding_named(&self, name: &str, declared: &dyn Fn(BindingId) -> bool) -> BindingId {
        self.program
            .bindings
            .ids()
            .find(|binding| {
                self.program.bindings.get(*binding).name() == name && declared(*binding)
            })
            .expect("the source declares the name")
    }
}
