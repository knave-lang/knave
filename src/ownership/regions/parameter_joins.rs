use rustc_hash::FxHashSet;

use super::body_flows::BodyFlows;
use super::lambda_flows::FunctionFlows;
use super::region_rules::RegionRules;
use super::store_sites::StoreSite;
use crate::core::BindingId;
use crate::ownership::TailCalls;
use crate::types::{StoredFlows, TypedArgument, TypedBinder, TypedExpr, TypedLambda};

pub(super) struct ParameterJoins<'r> {
    rules: &'r dyn RegionRules,
    body: BodyFlows<'r>,
}

impl<'r> ParameterJoins<'r> {
    pub(super) fn of_function(
        rules: &'r dyn RegionRules,
        lambda: &TypedLambda,
        calls: &TailCalls,
    ) -> FunctionFlows {
        let mut body = BodyFlows::new(rules, lambda, None);
        let flows = body.lambda_flows(lambda);
        let mut joins = Self { rules, body };
        FunctionFlows {
            flows,
            reassigned_parameters: joins.joined_by_tail_calls(lambda, calls),
        }
    }

    pub(super) fn of_stores(
        rules: &'r dyn RegionRules,
        lambda: &TypedLambda,
        sites: &[StoreSite],
    ) -> StoredFlows {
        let mut body = BodyFlows::new(rules, lambda, None);
        body.block(&lambda.body);
        let mut joins = Self { rules, body };
        let mut stored = StoredFlows::default();
        for site in sites {
            stored.extend(joins.joined_by_store(site));
        }
        stored
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
        if !self.rules.carries_borrow(&param.ty) || !self.rules.carries_borrow(&value.ty) {
            return FxHashSet::default();
        }
        let mut sources = self.body.bound(value);
        sources.remove(&param.binding);
        if !sources.is_empty() {
            sources.insert(param.binding);
        }
        sources
    }

    fn joined_by_store(&mut self, site: &StoreSite) -> StoredFlows {
        let values = self.body.bound(site.value);
        let targets = if values.is_empty() {
            FxHashSet::default()
        } else {
            self.body.bound(site.target)
        };
        StoredFlows { values, targets }
    }
}
