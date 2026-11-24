use rustc_hash::FxHashSet;

use super::lambda_flows::{FunctionFlows, LambdaFlows};
use super::left_out_parameters::LeftOutParameters;
use super::region_rules::RegionRules;
use super::result_borrows::ResultBorrows;
use super::result_held_parameters::ResultHeldParameters;
use crate::core::BindingId;
use crate::types::{
    DefinitionTable, ParameterRole, RecordedClosures, Regions, ResultRegions, TypedDefinition,
    TypedExpr, TypedExprKind, TypedLambda, TypedProgram,
};

pub struct RegionRecord<'a> {
    program: &'a TypedProgram,
    analysis: ResultBorrows<'a>,
    held: ResultHeldParameters<'a>,
    left_out: LeftOutParameters<'a>,
    regions: Regions,
}

impl RegionRecord<'_> {
    pub fn of(
        program: &TypedProgram,
        definitions: &DefinitionTable<'_>,
        covered: &FxHashSet<BindingId>,
    ) -> Regions {
        let mut record = RegionRecord {
            program,
            analysis: ResultBorrows::new(program, definitions),
            held: ResultHeldParameters::new(program, definitions),
            left_out: LeftOutParameters::new(program),
            regions: Regions::default(),
        };
        for definition in covered
            .iter()
            .filter_map(|binding| definitions.definition(*binding))
        {
            record.definition(definition);
        }
        record.regions
    }

    fn definition(&mut self, definition: &TypedDefinition) {
        if let TypedExprKind::Fn(lambda) = &definition.value.kind
            && !lambda.params.is_empty()
        {
            self.function(definition.binding, lambda);
        }
        for child in definition.value.children() {
            self.borrowing_closures(child);
        }
    }

    fn borrowing_closures(&mut self, expr: &TypedExpr) {
        if let TypedExprKind::Fn(lambda) = &expr.kind
            && !lambda.params.is_empty()
            && RecordedClosures::new(self.program).includes(&expr.ty)
        {
            self.lambda(lambda, self.analysis.analyse(lambda));
        }
        for child in expr.children() {
            self.borrowing_closures(child);
        }
    }

    fn function(&mut self, function: BindingId, lambda: &TypedLambda) {
        let FunctionFlows {
            flows,
            reassigned_parameters,
        } = self.analysis.analyse_function(function, lambda);
        self.lambda(lambda, flows);
        let shared = self.analysis.analyse_shared_arguments(lambda);
        self.regions
            .record(ParameterRole::FlowsThroughSharedArguments, lambda, shared);
        self.regions
            .record(ParameterRole::HeldByResult, lambda, self.held.of(lambda));
        let kept = ResultRegions::of(&self.regions, lambda).kept();
        self.regions.record(
            ParameterRole::LeftOutOfResult,
            lambda,
            self.left_out.of(lambda, &kept),
        );
        self.regions
            .record(ParameterRole::Reassigned, lambda, reassigned_parameters);
        let stored = self.analysis.stores_of(function).flows.clone();
        self.regions
            .record(ParameterRole::StoredValue, lambda, stored.values);
        self.regions
            .record(ParameterRole::StoredTarget, lambda, stored.targets);
    }

    fn lambda(&mut self, lambda: &TypedLambda, flows: LambdaFlows) {
        self.regions
            .record(ParameterRole::FlowsToResult, lambda, flows.flows);
        self.regions
            .record(ParameterRole::HigherRanked, lambda, flows.higher_ranked);
    }
}
