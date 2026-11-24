use super::recorded_ties::RecordedTies;
use super::result_source::ResultSource;
use crate::types::{
    CalledFunction, DefinitionTable, ParameterRole, PositionTie, Regions, TemplateTies, Type,
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
                let tie = ties.tie_at(position)?;
                Some(ResultSource {
                    tie: self.kept_as(tie, parameter, &argument.value),
                    argument: &argument.value,
                })
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

    fn kept_as(&self, tie: PositionTie, parameter: &Type, argument: &TypedExpr) -> PositionTie {
        let lends_itself =
            parameter.is_reference() && !self.program.type_holds_borrow(argument.ty.referent());
        match tie {
            PositionTie::Lent if !lends_itself => PositionTie::Held,
            tie => tie,
        }
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
