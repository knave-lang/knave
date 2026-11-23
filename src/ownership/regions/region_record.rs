use rustc_hash::FxHashSet;

use super::lambda_flows::{FunctionFlows, LambdaFlows};
use super::region_rules::RegionRules;
use super::result_borrows::ResultBorrows;
use super::result_held_parameters::ResultHeldParameters;
use crate::core::BindingId;
use crate::modules::ModuleSources;
use crate::types::{
    DefinitionTable, ParameterRole, ProgramReach, RecordedClosures, Regions, TypedDefinition,
    TypedExpr, TypedExprKind, TypedLambda, TypedProgram,
};

pub struct RegionRecord<'a> {
    program: &'a TypedProgram,
    analysis: ResultBorrows<'a>,
    held: ResultHeldParameters<'a>,
    regions: Regions,
}

impl RegionRecord<'_> {
    pub fn of(
        program: &TypedProgram,
        definitions: &DefinitionTable<'_>,
        reach: &ProgramReach,
    ) -> Regions {
        let mut record = RegionRecord {
            program,
            analysis: ResultBorrows::new(program, definitions),
            held: ResultHeldParameters::new(program),
            regions: Regions::default(),
        };
        for definition in Self::covered(program, reach) {
            record.definition(definition);
        }
        record.regions
    }

    fn covered<'p>(program: &'p TypedProgram, reach: &ProgramReach) -> Vec<&'p TypedDefinition> {
        let reached: FxHashSet<BindingId> = reach.reached.iter().copied().collect();
        let reached = &reached;
        program
            .modules
            .iter()
            .flat_map(|module| {
                let user = !ModuleSources::is_prelude(&module.path);
                module
                    .definitions
                    .iter()
                    .filter(move |definition| user || reached.contains(&definition.binding))
            })
            .collect()
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
