use rustc_hash::FxHashMap;

use super::checker::TypeChecker;
use super::declared_generics::DeclaredGenerics;
use super::ty::{GenericId, Type};
use crate::core::{
    BindingArena, BindingId, BindingInfo, BindingKind, ClosureKind, ModuleId, PrimitiveType,
    ReferenceKind, TypeBinder, TypeBound, TypeExpr, TypeExprKind,
};
use crate::diagnostic::Diagnostic;
use crate::source::{SourceId, Span};

pub(super) struct WrittenTypeFixture {
    pub(super) checker: TypeChecker,
}

impl WrittenTypeFixture {
    pub(super) fn new() -> Self {
        Self {
            checker: TypeChecker::new(BindingArena::default()),
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

    fn binding(&mut self, name: &str, kind: BindingKind) -> BindingId {
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
                .map(|binding| TypeBound {
                    binding: *binding,
                    argument: None,
                })
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

    pub(super) fn declared_generics(&mut self) -> DeclaredGenerics {
        DeclaredGenerics::from(std::mem::take(&mut self.checker.generics))
    }
}
