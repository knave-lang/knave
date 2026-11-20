use rustc_hash::FxHashMap;

use super::RustTrait;
use super::checker::TypeChecker;
use super::collected_enum::CollectedEnum;
use super::collected_rust_trait::CollectedRustTrait;
use super::collected_struct::CollectedStruct;
use super::declared_generics::DeclaredGenerics;
use super::trait_fixture::TraitFixture;
use super::ty::{Bound, GenericId, Type};
use super::written_type_variables::WrittenTypeVariables;
use crate::core::{
    BindingArena, BindingId, BindingInfo, BindingKind, ClosureKind, FieldLabel, FieldSpelling,
    ModuleId, PrimitiveType, ReferenceKind, RustTraitDeclaration, TypeBinder, TypeBound, TypeExpr,
    TypeExprKind, VariantDeclaration, VariantPayload,
};
use crate::diagnostic::Diagnostic;
use crate::source::{SourceId, Span};

pub(super) struct WrittenTypeFixture {
    pub(super) checker: TypeChecker,
}

impl WrittenTypeFixture {
    pub(super) fn new() -> Self {
        Self {
            checker: TypeChecker::new::<crate::types::DroppedInlineSchemes>(BindingArena::default()),
        }
    }

    pub(super) fn span() -> Span {
        Span::new(SourceId::new(0), 0..0)
    }

    pub(super) fn type_variable(&mut self, name: &str) -> BindingId {
        self.binding(name, BindingKind::TypeVariable)
    }

    pub(super) fn associated_type(&mut self, name: &str) -> BindingId {
        self.binding(name, BindingKind::AssociatedType)
    }

    pub(super) fn lifetime(&mut self, name: &str) -> BindingId {
        self.binding(name, BindingKind::Lifetime)
    }

    pub(super) fn declared_struct(&mut self, name: &str, arity: usize) -> BindingId {
        let binding = self.binding(name, BindingKind::Struct(ModuleId::new(0)));
        self.checker.type_arities.insert(binding, arity);
        binding
    }

    pub(super) fn named_trait(&mut self, name: &str) -> BindingId {
        self.binding(name, BindingKind::Trait(ModuleId::new(0)))
    }

    pub(super) fn binding(&mut self, name: &str, kind: BindingKind) -> BindingId {
        self.checker
            .bindings
            .add(BindingInfo::new(name.into(), kind, None))
    }

    pub(super) fn expression(kind: TypeExprKind) -> TypeExpr {
        TypeExpr::new(kind, Self::span())
    }

    pub(super) fn integer() -> TypeExpr {
        Self::expression(TypeExprKind::Primitive(PrimitiveType::I64))
    }

    pub(super) fn named(binding: BindingId, arguments: Vec<TypeExpr>) -> TypeExpr {
        Self::expression(TypeExprKind::Named(binding, arguments))
    }

    pub(super) fn reference(kind: ReferenceKind, inner: TypeExpr) -> TypeExpr {
        Self::expression(TypeExprKind::Reference(kind, None, Box::new(inner)))
    }

    pub(super) fn function(parameters: Vec<TypeExpr>, result: TypeExpr) -> TypeExpr {
        Self::expression(TypeExprKind::Function(
            ClosureKind::Fn,
            parameters,
            Box::new(result),
        ))
    }

    pub(super) fn bounded(variable: BindingId, bounds: &[BindingId]) -> TypeBinder {
        TypeBinder {
            variable,
            bounds: bounds
                .iter()
                .map(|binding| TypeBound::new(*binding, None))
                .collect(),
            span: Self::span(),
            lifetime: false,
        }
    }

    pub(super) fn lifetime_binder(variable: BindingId) -> TypeBinder {
        TypeBinder {
            variable,
            bounds: Vec::new(),
            span: Self::span(),
            lifetime: true,
        }
    }

    pub(super) fn lower(
        &mut self,
        type_expr: &TypeExpr,
        scope: &FxHashMap<BindingId, GenericId>,
    ) -> Result<Type, Diagnostic> {
        self.checker.written_types().lower_type(type_expr, scope)
    }

    pub(super) fn lower_without_generics(
        &mut self,
        type_expr: &TypeExpr,
    ) -> Result<Type, Diagnostic> {
        self.checker
            .written_types()
            .lower_without_generics(type_expr)
    }

    pub(super) fn lower_bound(&mut self, bound: &TypeBound) -> Result<Bound, Diagnostic> {
        self.checker
            .written_types()
            .lower_type_bound(bound, &FxHashMap::default())
    }

    pub(super) fn bound_with_argument(binding: BindingId) -> TypeBound {
        TypeBound::new(binding, Some(Self::integer()))
    }

    pub(super) fn holding_trait(
        &mut self,
        rust_trait: impl FnOnce(&RustTraitDeclaration) -> RustTrait,
    ) -> BindingId {
        let binding = self.named_trait("shown");
        let declaration = TraitFixture::declaration(binding, Vec::new());
        let rust_trait = rust_trait(&declaration);
        let collected = CollectedRustTrait::new(&declaration, rust_trait);
        let traits = self.checker.declarations.open_builder().traits();
        traits.enter_rust_trait(collected, &mut self.checker.synthetic_bindings);
        binding
    }

    pub(super) fn field_label(name: &str) -> FieldLabel {
        FieldLabel {
            name: name.into(),
            span: Self::span(),
            spelling: FieldSpelling::Bare,
        }
    }

    pub(super) fn variant(&mut self, name: &str, payload: VariantPayload) -> VariantDeclaration {
        VariantDeclaration {
            binding: self.binding(name, BindingKind::Local),
            name: name.into(),
            payload,
            span: Self::span(),
        }
    }

    pub(super) fn type_variables(&self, type_expr: &TypeExpr) -> Vec<BindingId> {
        WrittenTypeVariables::new(&self.checker.bindings).of(type_expr)
    }

    pub(super) fn enter_struct(&mut self, binding: BindingId, collected: CollectedStruct) {
        let types = self.checker.declarations.open_builder().types();
        types.enter_struct(binding, collected, &mut self.checker.diagnostics);
    }

    pub(super) fn enter_enum(&mut self, binding: BindingId, collected: CollectedEnum) {
        let types = self.checker.declarations.open_builder().types();
        types.enter_enum(binding, collected, &mut self.checker.diagnostics);
    }

    pub(super) fn declared_generics(&mut self) -> DeclaredGenerics {
        DeclaredGenerics::from(std::mem::take(&mut self.checker.generics))
    }
}
