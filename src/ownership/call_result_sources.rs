use super::recorded_ties::RecordedTies;
use super::result_source::{PassedArgument, ResultSource};
use crate::types::{
    CalledFunction, DefinitionTable, ParameterRole, Regions, TemplateTies, Type, TypedExpr,
    TypedExprKind, TypedProgram,
};

pub struct CallResultSources<'p> {
    program: &'p TypedProgram,
    definitions: &'p DefinitionTable<'p>,
    regions: &'p Regions,
}

impl<'p> CallResultSources<'p> {
    pub fn new(
        program: &'p TypedProgram,
        definitions: &'p DefinitionTable<'p>,
        regions: &'p Regions,
    ) -> Self {
        Self {
            program,
            definitions,
            regions,
        }
    }

    pub fn of<'e>(&self, call: &'e TypedExpr) -> Option<Vec<ResultSource<'e>>> {
        let TypedExprKind::Call(callee, arguments) = &call.kind else {
            return None;
        };
        let (ties, parameters) = match self.definitions.called_function(call) {
            Some(called) => (
                self.function_ties(&called),
                Self::lambda_parameters(&called),
            ),
            None => {
                let Type::Function(_, parameters, _) = &callee.ty else {
                    return None;
                };
                (
                    RecordedTies::new(self.program).of(callee)?,
                    parameters.clone(),
                )
            }
        };
        let sources = parameters
            .iter()
            .zip(arguments)
            .enumerate()
            .filter_map(|(position, (parameter, argument))| {
                let passed = PassedArgument {
                    parameter,
                    argument: &argument.value,
                };
                Some(ResultSource::kept_by_call(
                    ties.tie_at(position)?,
                    passed,
                    self.program,
                ))
            })
            .collect();
        Some(sources)
    }

    fn lambda_parameters(called: &CalledFunction) -> Vec<Type> {
        called
            .lambda
            .params
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect()
    }

    fn function_ties(&self, called: &CalledFunction) -> TemplateTies {
        let flows = self.regions.of(ParameterRole::FlowsToResult, called.lambda);
        let mut lent: Vec<usize> = called
            .lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| flows.contains(&parameter.binding))
            .map(|(position, _)| position)
            .collect();
        lent.extend(self.signature_lent(called));
        TemplateTies::new(&lent, &[])
    }

    fn signature_lent(&self, called: &CalledFunction) -> Vec<usize> {
        let Some(result) = called.lambda.body.tail().map(|tail| &tail.ty) else {
            return Vec::new();
        };
        TemplateTies::lent_positions(&Self::lambda_parameters(called), result)
    }
}
