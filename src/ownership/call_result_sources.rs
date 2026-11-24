use super::recorded_ties::RecordedTies;
use super::result_source::{PassedArgument, ResultSource};
use crate::types::{
    CalledFunction, DefinitionTable, ParameterRole, Regions, TemplateTies, Type, TypedExpr,
    TypedExprKind, TypedLambda, TypedProgram,
};

struct CalleeTies<'t> {
    ties: TemplateTies,
    parameters: Vec<&'t Type>,
}

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
        let TypedExprKind::Call(_, arguments) = &call.kind else {
            return None;
        };
        let signature = self.callee_signature(call)?;
        let sources = signature
            .parameters
            .iter()
            .zip(arguments)
            .enumerate()
            .filter_map(|(position, (parameter, argument))| {
                let passed = PassedArgument {
                    parameter,
                    argument: &argument.value,
                };
                Some(ResultSource::kept_by_call(
                    signature.ties.tie_at(position)?,
                    passed,
                    self.program,
                ))
            })
            .collect();
        Some(sources)
    }

    fn callee_signature<'t>(&'t self, call: &'t TypedExpr) -> Option<CalleeTies<'t>> {
        match self.definitions.called_function(call) {
            Some(called) => Some(CalleeTies {
                ties: self.function_ties(&called),
                parameters: Self::lambda_parameters(called.lambda),
            }),
            None => self.template_signature(call),
        }
    }

    fn template_signature<'t>(&self, call: &'t TypedExpr) -> Option<CalleeTies<'t>> {
        let TypedExprKind::Call(callee, _) = &call.kind else {
            return None;
        };
        let Type::Function(_, parameters, _) = &callee.ty else {
            return None;
        };
        Some(CalleeTies {
            ties: RecordedTies::new(self.program).of(callee)?,
            parameters: parameters.iter().collect(),
        })
    }

    fn lambda_parameters(lambda: &TypedLambda) -> Vec<&Type> {
        lambda
            .params
            .iter()
            .map(|parameter| &parameter.ty)
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
        let parameters = Self::lambda_parameters(called.lambda);
        TemplateTies::lent_positions(&parameters, result).unwrap_or_default()
    }
}
