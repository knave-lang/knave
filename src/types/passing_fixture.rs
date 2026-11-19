use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::argument_passing::ArgumentPassing;
use super::borrow_rules::BorrowRules;
use super::copy_requirement::CopyRequirement;
use super::declarations_fixture::DeclarationsFixture;
use super::deferred_decisions::DeferredDecisions;
use super::passing_facts::PassingFacts;
use super::passing_tables::PassingTables;
use super::pending::PendingBorrow;
use super::reopened_parameters::ReopenedParameters;
use super::ty::{Bound, Type};
use super::typed::{Borrow, TypedExpr, TypedExprKind, TypedLeaf};
use super::unifier::{LiteralKind, Unifier};
use crate::core::{BindingId, PrimitiveType, ReferenceKind};
use crate::source::{SourceId, Span};

pub(super) struct PassingFixture {
    declarations: DeclarationsFixture,
    field_readers: FxHashMap<BindingId, Rc<str>>,
    unifier: Unifier,
    deferred: DeferredDecisions,
}

impl PassingFixture {
    const SOURCE: &'static str = "
        (defn main () (do))";

    pub(super) fn new() -> Self {
        Self {
            declarations: DeclarationsFixture::new(Self::SOURCE),
            field_readers: FxHashMap::default(),
            unifier: Unifier::default(),
            deferred: DeferredDecisions::default(),
        }
    }

    pub(super) fn argument_passing(&mut self) -> ArgumentPassing<'_> {
        let (tables, unifier, deferred) = self.split();
        ArgumentPassing::new(tables, unifier, deferred)
    }

    pub(super) fn rules(&mut self) -> BorrowRules<'_, '_> {
        let (tables, unifier, _) = self.split();
        BorrowRules::new(tables, unifier)
    }

    pub(super) fn passed(&mut self, parameter: &Type, argument: &Type) -> Borrow {
        self.rules()
            .pass_types(parameter, argument, Self::span())
            .expect("the types fit")
    }

    pub(super) fn passed_to_variable_bounded_by(
        &mut self,
        bound: Bound,
        argument: &Type,
    ) -> Borrow {
        let parameter = self.unifier.fresh();
        let Type::Var(var) = parameter else {
            unreachable!("fresh makes a variable")
        };
        self.unifier.add_bound(var, bound);
        self.passed(&parameter, argument)
    }

    pub(super) fn pending_borrow(var: &Type, argument: Type) -> PendingBorrow {
        let Type::Var(var) = var else {
            unreachable!("a pending borrow waits on a variable")
        };
        PendingBorrow {
            var: *var,
            argument,
            span: Self::span(),
        }
    }

    pub(super) fn reopened_parameters(&mut self) -> ReopenedParameters<'_> {
        let (tables, unifier, deferred) = self.split();
        ReopenedParameters::new(tables, unifier, deferred)
    }

    pub(super) fn copy_requirement(&mut self) -> CopyRequirement<'_> {
        let (tables, unifier, _) = self.split();
        CopyRequirement::new(tables.copy_trait, unifier)
    }

    pub(super) fn facts(&mut self) -> PassingFacts<'_> {
        let (tables, unifier, _) = self.split();
        PassingFacts::new(tables, unifier)
    }

    pub(super) fn unifier(&mut self) -> &mut Unifier {
        &mut self.unifier
    }

    pub(super) fn deferred(&self) -> &DeferredDecisions {
        &self.deferred
    }

    pub(super) fn copy_trait(&self) -> BindingId {
        self.trait_named("copy")
    }

    pub(super) fn trait_named(&self, name: &str) -> BindingId {
        self.declarations.trait_named(name)
    }

    pub(super) fn literal(&mut self) -> Type {
        self.unifier.fresh_literal(LiteralKind::Integer)
    }

    pub(super) fn local(&self, ty: Type) -> TypedExpr {
        let main = self.declarations.binding("main");
        Self::expression(TypedExprKind::Leaf(TypedLeaf::Local(main)), ty)
    }

    pub(super) fn call_result(&self, ty: Type) -> TypedExpr {
        let main = self.declarations.binding("main");
        Self::expression(TypedExprKind::Leaf(TypedLeaf::Global(main)), ty)
    }

    pub(super) fn written_borrow(&self, ty: Type) -> TypedExpr {
        let borrowed = Box::new(self.local(ty.clone()));
        let kind = TypedExprKind::Borrow(ReferenceKind::Shared, borrowed);
        Self::expression(kind, Type::shared(ty))
    }

    pub(super) fn int() -> Type {
        Type::Primitive(PrimitiveType::I64)
    }

    pub(super) fn string() -> Type {
        Type::Primitive(PrimitiveType::String)
    }

    pub(super) fn str() -> Type {
        Type::Primitive(PrimitiveType::Str)
    }

    pub(super) fn span() -> Span {
        Span::new(SourceId::new(0), 0..0)
    }

    fn expression(kind: TypedExprKind, ty: Type) -> TypedExpr {
        TypedExpr {
            kind,
            ty,
            span: Self::span(),
        }
    }

    fn split(&mut self) -> (PassingTables<'_>, &mut Unifier, &mut DeferredDecisions) {
        let tables = PassingTables {
            declarations: self.declarations.declarations(),
            bindings: self.declarations.program_bindings(),
            generics: self.declarations.generics(),
            single_field_readers: &self.field_readers,
            copy_trait: Some(self.declarations.trait_named("copy")),
        };
        (tables, &mut self.unifier, &mut self.deferred)
    }
}
