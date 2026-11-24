use super::recorded_ties::RecordedTies;
use super::result_source::ResultSource;
use super::tied_parts::TiedParts;
use crate::types::{
    CalledFunction, DefinitionTable, ParameterRole, Regions, TemplateTies, Type, TypedExpr,
    TypedExprKind, TypedLambda, TypedProgram,
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
        let sources = arguments
            .iter()
            .enumerate()
            .filter_map(|(position, argument)| {
                let tie = ties.tie_at(position)?;
                Some(ResultSource::kept_by_call(
                    tie,
                    &argument.value,
                    self.program,
                ))
            })
            .collect();
        Some(sources)
    }

    fn function_ties(&self, called: &CalledFunction) -> TemplateTies {
        let flows = self.regions.of(ParameterRole::FlowsToResult, called.lambda);
        let parameters = &called.lambda.params;
        let mut lent: Vec<usize> = parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| flows.contains(&parameter.binding))
            .map(|(position, _)| position)
            .collect();
        if let Some(result) = called.lambda.body.tail().map(|tail| &tail.ty) {
            lent.extend(self.signature_lent(called.lambda, result));
            let tied = TiedParts::of_result(result);
            lent.retain(|position| !tied.ties_only_part_of(&parameters[*position].ty));
        }
        TemplateTies::new(&lent, &[])
    }

    fn signature_lent(&self, lambda: &TypedLambda, result: &Type) -> Vec<usize> {
        let lending = self.lending_parameters(lambda);
        let types: Vec<&Type> = lending.iter().map(|(_, ty)| *ty).collect();
        TemplateTies::lent_positions(&types, result)
            .unwrap_or_default()
            .into_iter()
            .map(|index| lending[index].0)
            .collect()
    }

    fn lending_parameters<'l>(&self, lambda: &'l TypedLambda) -> Vec<(usize, &'l Type)> {
        let left_out = self.regions.of(ParameterRole::LeftOutOfResult, lambda);
        lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| !left_out.contains(&parameter.binding))
            .map(|(position, parameter)| (position, &parameter.ty))
            .collect()
    }
}
