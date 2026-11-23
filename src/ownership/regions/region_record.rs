use rustc_hash::FxHashSet;

use super::lambda_flows::{FunctionFlows, LambdaFlows};
use super::result_borrows::ResultBorrows;
use crate::core::BindingId;
use crate::modules::ModuleSources;
use crate::ownership::local_borrows::LocalBorrows;
use crate::types::{
    DefinitionTable, ParameterRole, Regions, TypedDefinition, TypedExprKind, TypedLambda,
    TypedProgram,
};

pub struct RegionRecord<'a> {
    analysis: ResultBorrows<'a>,
    regions: Regions,
}

impl RegionRecord<'_> {
    pub fn of(program: &TypedProgram) -> Regions {
        let definitions = DefinitionTable::of(program);
        let mut record = RegionRecord {
            analysis: ResultBorrows::new(program, &definitions),
            regions: Regions::default(),
        };
        for definition in Self::covered(program) {
            record.definition(definition);
        }
        record.regions
    }

    fn covered(program: &TypedProgram) -> Vec<&TypedDefinition> {
        let reached: FxHashSet<BindingId> =
            program.ownership.reach.reached.iter().copied().collect();
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
        if let TypedExprKind::Fn(lambda) = &definition.value.kind {
            self.function(definition.binding, lambda);
        }
        for closure in LocalBorrows::closures_in(&definition.value) {
            let flows = self.analysis.analyse(closure);
            self.lambda(closure, flows);
        }
    }

    fn function(&mut self, function: BindingId, lambda: &TypedLambda) {
        let FunctionFlows {
            flows,
            reassigned_parameters,
        } = self.analysis.analyse_function(function, lambda);
        self.lambda(lambda, flows);
        self.regions
            .record(ParameterRole::Reassigned, reassigned_parameters);
        let stored = self.analysis.definition_stores(function).flows.clone();
        self.regions
            .record(ParameterRole::StoredValue, stored.values);
        self.regions
            .record(ParameterRole::StoredTarget, stored.targets);
    }

    fn lambda(&mut self, lambda: &TypedLambda, flows: LambdaFlows) {
        self.regions
            .record(ParameterRole::FlowsToResult, flows.flows);
        self.regions
            .record(ParameterRole::HigherRanked, flows.higher_ranked);
        let shared = self.analysis.analyse_shared_arguments(lambda).flows;
        self.regions
            .record(ParameterRole::FlowsThroughSharedArguments, shared);
    }
}
