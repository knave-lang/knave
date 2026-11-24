use super::recorded_ties::RecordedTies;
use super::result_source::ResultSource;
use crate::types::{
    CalledFunction, DefinitionTable, ParameterRole, Regions, TemplateTies, Type, TypedArgument,
    TypedExpr, TypedExprKind, TypedProgram,
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
        let ties = match self.definitions.called_function(call) {
            Some(called) => self.function_ties(&called),
            None => RecordedTies::new(self.program).of(callee)?,
        };
        Some(Self::tied(&ties, arguments))
    }

    fn tied<'e>(ties: &TemplateTies, arguments: &'e [TypedArgument]) -> Vec<ResultSource<'e>> {
        arguments
            .iter()
            .enumerate()
            .filter_map(|(position, argument)| {
                Some(ResultSource {
                    tie: ties.tie_at(position)?,
                    argument: &argument.value,
                })
            })
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
        let parameters: Vec<Type> = called
            .lambda
            .params
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect();
        TemplateTies::lent_positions(&parameters, result)
    }
}
