use rustc_hash::FxHashSet;

use super::lambda_flows::{FunctionFlows, LambdaFlows};
use super::result_borrows::ResultBorrows;
use crate::core::BindingId;
use crate::modules::ModuleSources;
use crate::types::{
    DefinitionTable, ParameterRole, Regions, Type, TypedDefinition, TypedExpr, TypedExprKind,
    TypedLambda, TypedProgram,
};

pub struct RegionRecord<'a> {
    program: &'a TypedProgram,
    analysis: ResultBorrows<'a>,
    impl_methods: FxHashSet<BindingId>,
    regions: Regions,
}

impl RegionRecord<'_> {
    pub fn of(program: &TypedProgram) -> Regions {
        let definitions = DefinitionTable::of(program);
        let mut record = RegionRecord {
            program,
            analysis: ResultBorrows::new(program, &definitions),
            impl_methods: Self::impl_methods(program),
            regions: Regions::default(),
        };
        for definition in Self::covered(program) {
            record.definition(definition);
        }
        record.regions
    }

    fn impl_methods(program: &TypedProgram) -> FxHashSet<BindingId> {
        program
            .declarations
            .traits()
            .impls()
            .iter()
            .flat_map(|implementation| &implementation.methods)
            .map(|method| method.binding)
            .collect()
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
        for child in definition.value.children() {
            self.borrowing_closures(child);
        }
    }

    fn borrowing_closures(&mut self, expr: &TypedExpr) {
        if let TypedExprKind::Fn(lambda) = &expr.kind
            && self.returns_borrow(&expr.ty)
        {
            self.lambda(self.analysis.analyse(lambda));
        }
        for child in expr.children() {
            self.borrowing_closures(child);
        }
    }

    fn returns_borrow(&self, ty: &Type) -> bool {
        matches!(ty, Type::Function(_, _, result) if self.program.function_type_returns_borrow(result))
    }

    fn function(&mut self, function: BindingId, lambda: &TypedLambda) {
        let FunctionFlows {
            flows,
            reassigned_parameters,
        } = self.analysis.analyse_function(function, lambda);
        self.lambda(flows);
        if self.reads_shared_arguments(function, lambda) {
            let shared = self.analysis.analyse_shared_arguments(lambda).flows;
            self.regions
                .record(ParameterRole::FlowsThroughSharedArguments, shared);
        }
        self.regions
            .record(ParameterRole::Reassigned, reassigned_parameters);
        let stored = self.analysis.definition_stores(function).flows.clone();
        self.regions
            .record(ParameterRole::StoredValue, stored.values);
        self.regions
            .record(ParameterRole::StoredTarget, stored.targets);
    }

    fn reads_shared_arguments(&self, function: BindingId, lambda: &TypedLambda) -> bool {
        let modes = &self.program.ownership.parameter_modes;
        self.impl_methods.contains(&function)
            || lambda
                .parameter_bindings()
                .any(|parameter| modes.changes(parameter))
    }

    fn lambda(&mut self, flows: LambdaFlows) {
        self.regions
            .record(ParameterRole::FlowsToResult, flows.flows);
        self.regions
            .record(ParameterRole::HigherRanked, flows.higher_ranked);
    }
}
