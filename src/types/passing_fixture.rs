use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::argument_passing::ArgumentPassing;
use super::borrow_rules::BorrowRules;
use super::callee_borrows::CalleeBorrows;
use super::copy_requirement::CopyRequirement;
use super::declarations_fixture::DeclarationsFixture;
use super::deferred_decisions::DeferredDecisions;
use super::group_depths::GroupDepths;
use super::passing_facts::PassingFacts;
use super::passing_tables::PassingTables;
use super::pending::PendingBorrow;
use super::place_types::PlaceTypes;
use super::reference_depth::ReferenceDepth;
use super::reopened_parameters::ReopenedParameters;
use super::shared_group::SharedGroup;
use super::shared_passing_order::SharedPassingOrder;
use super::shared_variable_group::SharedVariableGroup;
use super::string_views::StringViews;
use super::templates::TemplateBorrows;
use super::ty::{Bound, Type};
use super::typed::{Borrow, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf};
use super::unifier::{LiteralKind, Unifier};
use crate::core::{BindingId, PrimitiveType, ReferenceKind, RustTemplate, TypeExpr, TypeExprKind};
use crate::source::{SourceId, Span};

pub(super) struct PassingFixture {
    declarations: DeclarationsFixture,
    field_readers: FxHashMap<BindingId, Rc<str>>,
    unifier: Unifier,
    deferred: DeferredDecisions,
    templates: TemplateBorrows,
}

impl PassingFixture {
    const SOURCE: &'static str = "
        (defn main () (do))";
    pub(super) const BORROWING_SECOND: &'static str = "{0} == {&1}";
    pub(super) const BORROWING_BOTH: &'static str = "{&0} == {&1}";

    pub(super) fn new() -> Self {
        Self {
            declarations: DeclarationsFixture::new(Self::SOURCE),
            field_readers: FxHashMap::default(),
            unifier: Unifier::default(),
            deferred: DeferredDecisions::default(),
            templates: TemplateBorrows::default(),
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

    pub(super) fn written_mut_borrow(&self, ty: Type) -> TypedExpr {
        let borrowed = Box::new(self.local(ty.clone()));
        let kind = TypedExprKind::Borrow(ReferenceKind::Mutable, borrowed);
        Self::expression(kind, Type::mutable(ty))
    }

    pub(super) fn string_views(&mut self, view: Option<BindingId>) -> StringViews<'_> {
        let (tables, unifier, _) = self.split();
        let unifier = &*unifier;
        StringViews::new(
            unifier,
            PlaceTypes::new(unifier, tables.declared_types()),
            view,
        )
    }

    pub(super) fn argument(&self, value: TypedExpr) -> TypedArgument {
        TypedArgument {
            value,
            borrow: Borrow::None,
        }
    }

    pub(super) fn view_binding(&self) -> BindingId {
        self.declarations.binding("main")
    }

    pub(super) fn shared_group(&mut self, members: &[usize]) -> SharedGroup {
        let vars: Vec<_> = (0..=members.iter().copied().max().unwrap_or(0))
            .map(|_| self.fresh_var())
            .collect();
        let mut group = SharedGroup::new(vars[0]);
        for (position, member) in members.iter().enumerate() {
            group.add_position(vars[*member], position);
        }
        group
    }

    pub(super) fn shared_variable_group<'f>(
        &'f mut self,
        group: &'f SharedGroup,
    ) -> SharedVariableGroup<'f> {
        let (tables, unifier, _) = self.split();
        SharedVariableGroup::new(group, tables, unifier)
    }

    pub(super) fn shared_passing_order<'f>(
        &'f mut self,
        callee: Option<&'f TypedExpr>,
    ) -> SharedPassingOrder<'f> {
        self.passing_order_for(callee)
    }

    fn passing_order_for<'f>(&'f self, callee: Option<&'f TypedExpr>) -> SharedPassingOrder<'f> {
        let tables = Self::tables_of(&self.declarations, &self.field_readers);
        let places = PlaceTypes::new(&self.unifier, tables.declared_types());
        SharedPassingOrder::new(places, &self.unifier, self.callee_borrows(callee))
    }

    pub(super) fn callee_borrows<'f>(&'f self, callee: Option<&'f TypedExpr>) -> CalleeBorrows<'f> {
        CalleeBorrows::new(callee, &self.templates)
    }

    pub(super) fn shared_target(&mut self, referent: Type, depths: &[usize]) -> ReferenceDepth {
        self.shared_target_at(None, referent, depths)
    }

    pub(super) fn shared_target_for(
        &mut self,
        callee: &TypedExpr,
        referent: Type,
        depths: &[usize],
    ) -> ReferenceDepth {
        self.shared_target_at(Some(callee), referent, depths)
    }

    fn shared_target_at(
        &mut self,
        callee: Option<&TypedExpr>,
        referent: Type,
        depths: &[usize],
    ) -> ReferenceDepth {
        let group = self.shared_group(&vec![0; depths.len()]);
        let depths = GroupDepths {
            positions: (0..depths.len()).collect(),
            resolved: vec![referent.clone(); depths.len()],
            depths: depths.to_vec(),
        };
        let tables = Self::tables_of(&self.declarations, &self.field_readers);
        let order = self.passing_order_for(callee);
        SharedVariableGroup::new(&group, tables, &self.unifier)
            .target_depth(&referent, &depths, &order)
    }

    pub(super) fn template_call(text: &str) -> TypedExpr {
        let template = RustTemplate {
            template: Rc::from(text),
            type_expr: TypeExpr::new(TypeExprKind::Hole, Self::span()),
            clauses: Vec::new(),
        };
        Self::expression(
            TypedExprKind::Leaf(TypedLeaf::Rust(Rc::new(template))),
            Type::Primitive(PrimitiveType::Bool),
        )
    }

    fn fresh_var(&mut self) -> super::ty::TypeVar {
        let Type::Var(var) = self.unifier.fresh() else {
            unreachable!("fresh makes a variable")
        };
        var
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
        let tables = Self::tables_of(&self.declarations, &self.field_readers);
        (tables, &mut self.unifier, &mut self.deferred)
    }

    fn tables_of<'f>(
        declarations: &'f DeclarationsFixture,
        field_readers: &'f FxHashMap<BindingId, Rc<str>>,
    ) -> PassingTables<'f> {
        PassingTables {
            declarations: declarations.declarations(),
            bindings: declarations.program_bindings(),
            generics: declarations.generics(),
            single_field_readers: field_readers,
            copy_trait: Some(declarations.trait_named("copy")),
        }
    }
}
