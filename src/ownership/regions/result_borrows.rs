use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::callee_parameters::{CalleeParameters, OpenCallees};
use super::definition_lookup::DefinitionLookup;
use super::lambda_flows::{FunctionFlows, LambdaFlows};
use super::local_borrow_calls::{LocalBorrowCalls, PassedLocal};
use super::store_sites::{StoreSite, StoreSites};
use super::stored_flows::{DefinitionStores, StoredFlows};
use crate::core::BindingId;
use crate::ownership::TailCalls;
use crate::types::{
    Type, TypedArgument, TypedBinder, TypedBlock, TypedBodyItem, TypedControl, TypedExpr,
    TypedExprKind, TypedLambda, TypedLeaf, TypedMatch, TypedProgram,
};

pub struct ResultBorrows<'a> {
    program: &'a TypedProgram,
    definitions: &'a dyn DefinitionLookup,
    parameters_by_callee: RefCell<FxHashMap<BindingId, CalleeParameters>>,
    open_callees: RefCell<OpenCallees>,
    stores_by_definition: RefCell<FxHashMap<BindingId, Rc<DefinitionStores>>>,
    no_stores: Rc<DefinitionStores>,
}

struct Visit {
    parameters: CalleeParameters,
    earliest_reached: Option<usize>,
}

impl<'a> ResultBorrows<'a> {
    pub fn new(program: &'a TypedProgram, definitions: &'a dyn DefinitionLookup) -> Self {
        Self {
            program,
            definitions,
            parameters_by_callee: RefCell::new(FxHashMap::default()),
            open_callees: RefCell::new(OpenCallees::default()),
            stores_by_definition: RefCell::new(FxHashMap::default()),
            no_stores: Rc::default(),
        }
    }

    pub fn analyse(&self, lambda: &TypedLambda) -> LambdaFlows {
        BodyFlows::of(self, lambda, None)
    }

    pub fn analyse_shared_arguments(&self, lambda: &TypedLambda) -> LambdaFlows {
        BodyFlows::of_shared_arguments(self, lambda)
    }

    fn stored_at_sites(&self, lambda: &TypedLambda) -> StoredFlows {
        let sites = StoreSites::of(self, lambda);
        if sites.is_empty() {
            return StoredFlows::default();
        }
        BodyFlows::joined_by_stores(self, lambda, &sites)
    }

    pub fn analyse_function(&self, function: BindingId, lambda: &TypedLambda) -> FunctionFlows {
        BodyFlows::of_function(self, lambda, &TailCalls::of(Some(function), lambda))
    }

    pub fn definition_stores(&self, definition: BindingId) -> Rc<DefinitionStores> {
        if let Some(stores) = self.stores_by_definition.borrow().get(&definition) {
            return Rc::clone(stores);
        }
        let stores = match self.callee_lambda(definition) {
            Some(lambda) if self.may_receive_store(lambda) => {
                self.receiving_stores(definition, lambda)
            }
            _ => Rc::clone(&self.no_stores),
        };
        self.remember_stores(definition, &stores);
        stores
    }

    fn receiving_stores(
        &self,
        definition: BindingId,
        lambda: &TypedLambda,
    ) -> Rc<DefinitionStores> {
        self.remember_stores(definition, &self.no_stores);
        let flows = self.stored_at_sites(lambda);
        let positions = flows.positions(lambda);
        Rc::new(DefinitionStores { flows, positions })
    }

    fn remember_stores(&self, definition: BindingId, stores: &Rc<DefinitionStores>) {
        self.stores_by_definition
            .borrow_mut()
            .insert(definition, Rc::clone(stores));
    }

    pub(super) fn carries_borrow(&self, ty: &Type) -> bool {
        self.program.type_holds_borrow(ty) || ty.is_unsized()
    }

    pub(super) fn program(&self) -> &'a TypedProgram {
        self.program
    }

    fn may_receive_store(&self, lambda: &TypedLambda) -> bool {
        lambda
            .params
            .iter()
            .any(|param| self.referent_holds_borrow(&param.ty))
    }

    fn referent_holds_borrow(&self, ty: &Type) -> bool {
        match ty {
            Type::Ref(..) => false,
            Type::Mut(_, referent) => self.program.type_holds_borrow(referent),
            owned => self.program.type_holds_borrow(owned),
        }
    }

    fn cached_parameters(&self, callee: BindingId) -> Option<CalleeParameters> {
        self.parameters_by_callee.borrow().get(&callee).cloned()
    }

    fn open_order(&self, callee: BindingId) -> Option<usize> {
        self.open_callees.borrow().order(callee)
    }

    fn assume_parameters(&self, callee: BindingId) -> CalleeParameters {
        self.open_callees.borrow_mut().assume(callee)
    }

    fn visit(&self, callee: BindingId, lambda: &TypedLambda) -> Visit {
        let order = self.open_callees.borrow_mut().open(callee);
        let visit = self.proposed_visit(callee, lambda, order);
        if visit.earliest_reached == Some(order) {
            self.settle_group(order);
            return self.settled_visit(callee);
        }
        visit
    }

    fn proposed_visit(&self, callee: BindingId, lambda: &TypedLambda, order: usize) -> Visit {
        let analysed = BodyFlows::of(self, lambda, Some(order));
        let parameters = analysed.parameters(lambda);
        self.open_callees
            .borrow_mut()
            .propose(callee, parameters.clone());
        Visit {
            parameters,
            earliest_reached: analysed.earliest_reached,
        }
    }

    fn settled_visit(&self, callee: BindingId) -> Visit {
        Visit {
            parameters: self.cached_parameters(callee).unwrap_or_default(),
            earliest_reached: None,
        }
    }

    fn settle_group(&self, order: usize) {
        let group = self.open_callees.borrow().group_from(order);
        if self.open_callees.borrow().any_assumed(&group) {
            self.reach_fixed_point(&group);
        }
        let settled = self.open_callees.borrow_mut().close(&group);
        self.parameters_by_callee.borrow_mut().extend(settled);
    }

    fn reach_fixed_point(&self, group: &[BindingId]) {
        while self.widen_group(group) {}
    }

    fn widen_group(&self, group: &[BindingId]) -> bool {
        let grown: Vec<bool> = group
            .iter()
            .map(|callee| self.widen_member(*callee))
            .collect();
        grown.contains(&true)
    }

    fn widen_member(&self, callee: BindingId) -> bool {
        let Some(lambda) = self.callee_lambda(callee) else {
            return false;
        };
        let analysed = BodyFlows::of(self, lambda, None);
        let parameters = analysed.parameters(lambda);
        self.open_callees.borrow_mut().widen(callee, &parameters)
    }

    fn callee_lambda(&self, callee: BindingId) -> Option<&'a TypedLambda> {
        match &self.definitions.definition(callee)?.value.kind {
            TypedExprKind::Fn(lambda) => Some(lambda),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CallFlows {
    EveryArgument,
    SharedArguments,
}

struct BodyFlows<'r, 'a> {
    rules: &'r ResultBorrows<'a>,
    calls: CallFlows,
    parameters: FxHashSet<BindingId>,
    aliases: FxHashMap<BindingId, FxHashSet<BindingId>>,
    earliest_reached: Option<usize>,
}

impl<'r, 'a> BodyFlows<'r, 'a> {
    fn of(rules: &'r ResultBorrows<'a>, lambda: &TypedLambda, order: Option<usize>) -> LambdaFlows {
        Self::new(rules, lambda, order).lambda_flows(lambda)
    }

    fn of_shared_arguments(rules: &'r ResultBorrows<'a>, lambda: &TypedLambda) -> LambdaFlows {
        let mut borrows = Self {
            calls: CallFlows::SharedArguments,
            ..Self::new(rules, lambda, None)
        };
        borrows.lambda_flows(lambda)
    }

    fn of_function(
        rules: &'r ResultBorrows<'a>,
        lambda: &TypedLambda,
        calls: &TailCalls,
    ) -> FunctionFlows {
        let mut borrows = Self::new(rules, lambda, None);
        let flows = borrows.lambda_flows(lambda);
        FunctionFlows {
            flows,
            reassigned_parameters: borrows.joined_by_tail_calls(lambda, calls),
        }
    }

    fn lambda_flows(&mut self, lambda: &TypedLambda) -> LambdaFlows {
        let mut flows = self.block(&lambda.body);
        let calls = LocalBorrowCalls::of(self.rules.program, lambda);
        let mut higher_ranked: FxHashSet<BindingId> = calls
            .callees
            .iter()
            .flat_map(|callee| self.local(*callee))
            .collect();
        flows.retain(|binding| !higher_ranked.contains(binding));
        higher_ranked.extend(self.passed_to_higher_ranked(&calls.passed));
        LambdaFlows {
            flows,
            higher_ranked,
            earliest_reached: self.earliest_reached,
        }
    }

    fn new(rules: &'r ResultBorrows<'a>, lambda: &TypedLambda, order: Option<usize>) -> Self {
        Self {
            rules,
            calls: CallFlows::EveryArgument,
            parameters: lambda.params.iter().map(|param| param.binding).collect(),
            aliases: FxHashMap::default(),
            earliest_reached: order,
        }
    }

    fn passed_to_higher_ranked(&mut self, passed: &[PassedLocal]) -> FxHashSet<BindingId> {
        let higher_ranked: Vec<&PassedLocal> = passed
            .iter()
            .filter(|passed| self.reaches_higher_ranked(passed))
            .collect();
        higher_ranked
            .into_iter()
            .flat_map(|passed| self.local(passed.local))
            .collect()
    }

    fn reaches_higher_ranked(&mut self, passed: &PassedLocal) -> bool {
        self.callee_parameters(passed.callee)
            .is_higher_ranked(passed.position)
    }

    fn joined_by_tail_calls(
        &mut self,
        lambda: &TypedLambda,
        calls: &TailCalls,
    ) -> FxHashSet<BindingId> {
        calls
            .arguments()
            .iter()
            .flat_map(|arguments| self.joined_by_tail_call(lambda, arguments))
            .collect()
    }

    fn joined_by_tail_call(
        &mut self,
        lambda: &TypedLambda,
        arguments: &[TypedArgument],
    ) -> FxHashSet<BindingId> {
        lambda
            .params
            .iter()
            .zip(arguments)
            .flat_map(|(param, argument)| self.joined_by_reassignment(param, &argument.value))
            .collect()
    }

    fn joined_by_reassignment(
        &mut self,
        param: &TypedBinder,
        value: &TypedExpr,
    ) -> FxHashSet<BindingId> {
        if !self.carries_borrow(&param.ty) || !self.carries_borrow(&value.ty) {
            return FxHashSet::default();
        }
        let mut sources = self.bound(value);
        sources.remove(&param.binding);
        if !sources.is_empty() {
            sources.insert(param.binding);
        }
        sources
    }

    fn joined_by_stores(
        rules: &'r ResultBorrows<'a>,
        lambda: &TypedLambda,
        sites: &[StoreSite],
    ) -> StoredFlows {
        let mut borrows = Self::new(rules, lambda, None);
        borrows.block(&lambda.body);
        let mut stored = StoredFlows::default();
        for site in sites {
            stored.extend(borrows.joined_by_store(site));
        }
        stored
    }

    fn joined_by_store(&mut self, site: &StoreSite) -> StoredFlows {
        let values = self.bound(site.value);
        let targets = if values.is_empty() {
            FxHashSet::default()
        } else {
            self.bound(site.target)
        };
        StoredFlows { values, targets }
    }

    fn block(&mut self, block: &TypedBlock) -> FxHashSet<BindingId> {
        for item in &block.items {
            if let TypedBodyItem::Let(binder, value) = item {
                let flows = self.bound(value);
                self.aliases.insert(binder.binding, flows);
            }
        }
        match block.items.last() {
            Some(TypedBodyItem::Expr(expr)) => self.value(expr),
            _ => FxHashSet::default(),
        }
    }

    fn bound(&mut self, value: &TypedExpr) -> FxHashSet<BindingId> {
        match value.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self.local(binding),
            _ => self.value(value),
        }
    }

    fn value(&mut self, expr: &TypedExpr) -> FxHashSet<BindingId> {
        if !self.carries_borrow(&expr.ty) {
            return FxHashSet::default();
        }
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self.local(*binding),
            TypedExprKind::Control(control) => self.control(control),
            TypedExprKind::Call(callee, arguments) => self.call(callee, arguments, expr),
            TypedExprKind::Borrow(_, place) => self.borrowed(place),
            _ => self.children(expr),
        }
    }

    fn borrowed(&mut self, place: &TypedExpr) -> FxHashSet<BindingId> {
        if let TypedExprKind::Deref(reference) = &place.kind {
            return self.value(reference);
        }
        let mut flows = self.value(place);
        if let Some(root) = place.place_root() {
            flows.extend(self.local(root));
        }
        flows
    }

    fn carries_borrow(&self, ty: &Type) -> bool {
        self.rules.carries_borrow(ty)
    }

    fn call(
        &mut self,
        callee: &TypedExpr,
        arguments: &[TypedArgument],
        call: &TypedExpr,
    ) -> FxHashSet<BindingId> {
        let mut flows = self.callee(callee, arguments);
        if self.calls == CallFlows::EveryArgument {
            flows.extend(self.children(call));
        }
        flows
    }

    fn callee(&mut self, callee: &TypedExpr, arguments: &[TypedArgument]) -> FxHashSet<BindingId> {
        match callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self.local(binding),
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => self.passed_on(binding, arguments),
            _ => FxHashSet::default(),
        }
    }

    fn passed_on(
        &mut self,
        callee: BindingId,
        arguments: &[TypedArgument],
    ) -> FxHashSet<BindingId> {
        let parameters = self.callee_parameters(callee);
        arguments
            .iter()
            .enumerate()
            .filter(|(position, _)| parameters.shares(*position))
            .flat_map(|(_, argument)| self.bound(&argument.value))
            .collect()
    }

    fn callee_parameters(&mut self, callee: BindingId) -> CalleeParameters {
        if let Some(cached) = self.rules.cached_parameters(callee) {
            return cached;
        }
        match self.rules.open_order(callee) {
            Some(order) => self.assumed_parameters(callee, order),
            None => self.visited_parameters(callee),
        }
    }

    fn assumed_parameters(&mut self, callee: BindingId, order: usize) -> CalleeParameters {
        self.reach(order);
        self.rules.assume_parameters(callee)
    }

    fn visited_parameters(&mut self, callee: BindingId) -> CalleeParameters {
        let Some(lambda) = self.rules.callee_lambda(callee) else {
            return CalleeParameters::default();
        };
        let visit = self.rules.visit(callee, lambda);
        if let Some(order) = visit.earliest_reached {
            self.reach(order);
        }
        visit.parameters
    }

    fn reach(&mut self, order: usize) {
        self.earliest_reached = Some(
            self.earliest_reached
                .map_or(order, |earliest| earliest.min(order)),
        );
    }

    fn control(&mut self, control: &TypedControl) -> FxHashSet<BindingId> {
        match control {
            TypedControl::Block(block) => self.block(block),
            TypedControl::If(_, then, otherwise) => self.branches(then, otherwise),
            TypedControl::Match(matched) => self.matched(matched),
        }
    }

    fn branches(&mut self, then: &TypedExpr, otherwise: &TypedExpr) -> FxHashSet<BindingId> {
        let mut flows = self.value(then);
        flows.extend(self.value(otherwise));
        flows
    }

    fn matched(&mut self, matched: &TypedMatch) -> FxHashSet<BindingId> {
        let checked = self.value(&matched.value);
        let mut flows = match self.calls {
            CallFlows::EveryArgument => checked,
            CallFlows::SharedArguments => self.lend_to_pattern_bindings(matched, &checked),
        };
        for arm in &matched.arms {
            flows.extend(self.block(&arm.body));
        }
        flows
    }

    fn lend_to_pattern_bindings(
        &mut self,
        matched: &TypedMatch,
        checked: &FxHashSet<BindingId>,
    ) -> FxHashSet<BindingId> {
        for arm in &matched.arms {
            for binder in arm.pattern.binders() {
                self.aliases.insert(binder.binding, checked.clone());
            }
        }
        FxHashSet::default()
    }

    fn children(&mut self, expr: &TypedExpr) -> FxHashSet<BindingId> {
        expr.children()
            .into_iter()
            .flat_map(|child| self.value(child))
            .collect()
    }

    fn local(&self, binding: BindingId) -> FxHashSet<BindingId> {
        if self.parameters.contains(&binding) {
            FxHashSet::from_iter([binding])
        } else {
            self.aliases.get(&binding).cloned().unwrap_or_default()
        }
    }
}
