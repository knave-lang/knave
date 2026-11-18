use super::collected_rust_trait::CollectedRustTrait;
use super::collected_trait::CollectedTrait;
use super::trait_declaration_collector::TraitDeclarationCollector;
use super::written_type_fixture::WrittenTypeFixture;
use crate::core::{
    BindingId, BindingKind, ImplementorRows, RustTraitDeclaration, TraitMethod, TypeExpr,
};

pub(super) struct TraitFixture {
    pub(super) types: WrittenTypeFixture,
}

impl TraitFixture {
    pub(super) fn new() -> Self {
        Self {
            types: WrittenTypeFixture::new(),
        }
    }

    pub(super) fn declaration(
        binding: BindingId,
        methods: Vec<TraitMethod>,
    ) -> RustTraitDeclaration {
        RustTraitDeclaration {
            binding,
            path: "Shown".into(),
            resolved_path: "Shown".into(),
            self_variable: binding,
            methods,
            help: None,
            requires: Vec::new(),
            structural: None,
            associated: Vec::new(),
            implementors: ImplementorRows::new(Vec::new()),
        }
    }

    pub(super) fn rust_trait_declaration(
        &mut self,
        methods: Vec<TraitMethod>,
    ) -> RustTraitDeclaration {
        let binding = self.types.named_trait("shown");
        Self::declaration(binding, methods)
    }

    pub(super) fn trait_method(
        &mut self,
        signature: TypeExpr,
        default: Option<BindingId>,
    ) -> TraitMethod {
        TraitMethod {
            binding: self.types.binding("method", BindingKind::Local),
            signature,
            default,
        }
    }

    pub(super) fn default_method(&mut self) -> BindingId {
        self.types.binding("default", BindingKind::Local)
    }

    pub(super) fn trait_collector(&mut self) -> TraitDeclarationCollector<'_> {
        TraitDeclarationCollector::new(self.types.checker.written_types())
    }

    pub(super) fn enter_trait(&mut self, collected: CollectedTrait) {
        let checker = &mut self.types.checker;
        collected.enter(
            checker.declarations.traits_mut(),
            &mut checker.checking_facts,
            &mut checker.synthetic_bindings,
        );
    }

    pub(super) fn enter_rust_trait(&mut self, collected: CollectedRustTrait<'_>) {
        let checker = &mut self.types.checker;
        collected.enter(
            checker.declarations.traits_mut(),
            &mut checker.synthetic_bindings,
        );
    }
}
