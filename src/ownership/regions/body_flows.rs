use rustc_hash::{FxHashMap, FxHashSet};

use super::callee_parameters::CalleeParameters;
use super::lambda_flows::LambdaFlows;
use super::local_borrow_calls::{LocalBorrowCalls, PassedLocal};
use super::region_rules::RegionRules;
use crate::core::BindingId;
use crate::types::{
    TypedArgument, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda,
    TypedLeaf, TypedMatch,
};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum CallFlows {
    EveryArgument,
    SharedArguments,
}

pub(super) struct BodyFlows<'r> {
    rules: &'r dyn RegionRules,
    calls: CallFlows,
    parameters: FxHashSet<BindingId>,
    aliases: FxHashMap<BindingId, FxHashSet<BindingId>>,
    earliest_reached: Option<usize>,
}

impl<'r> BodyFlows<'r> {
    pub(super) fn of(
        rules: &'r dyn RegionRules,
        lambda: &TypedLambda,
        order: Option<usize>,
    ) -> LambdaFlows {
        Self::new(rules, lambda, order).lambda_flows(lambda)
    }

    pub(super) fn of_shared_arguments(
        rules: &'r dyn RegionRules,
        lambda: &TypedLambda,
    ) -> LambdaFlows {
        let mut borrows = Self {
            calls: CallFlows::SharedArguments,
            ..Self::new(rules, lambda, None)
        };
        borrows.lambda_flows(lambda)
    }

    pub(super) fn lambda_flows(&mut self, lambda: &TypedLambda) -> LambdaFlows {
        let mut flows = self.block(&lambda.body);
        let calls = LocalBorrowCalls::of(self.rules.program(), lambda);
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

    pub(super) fn new(
        rules: &'r dyn RegionRules,
        lambda: &TypedLambda,
        order: Option<usize>,
    ) -> Self {
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
            .filter(|passed| {
                self.callee_parameters(passed.callee)
                    .is_higher_ranked(passed.position)
            })
            .collect();
        higher_ranked
            .into_iter()
            .flat_map(|passed| self.local(passed.local))
            .collect()
    }

    pub(super) fn block(&mut self, block: &TypedBlock) -> FxHashSet<BindingId> {
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

    pub(super) fn bound(&mut self, value: &TypedExpr) -> FxHashSet<BindingId> {
        match value.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self.local(binding),
            _ => self.value(value),
        }
    }

    fn value(&mut self, expr: &TypedExpr) -> FxHashSet<BindingId> {
        if !self.rules.carries_borrow(&expr.ty) {
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
        let visit = self.rules.callee_visit(callee);
        self.earliest_reached = self
            .earliest_reached
            .into_iter()
            .chain(visit.earliest_reached)
            .min();
        visit.parameters
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
