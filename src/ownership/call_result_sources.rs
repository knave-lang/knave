use super::result_source::ResultSource;
use crate::types::{
    CalledFunction, DefinitionTable, ParameterRole, Pipelines, Regions, Scheme, TemplateAuthor,
    TemplateTies, Type, TypedArgument, TypedExpr, TypedExprKind, TypedProgram,
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
            None => self
                .program
                .template_borrows
                .read_template(callee, None, |_, ties| ties.cloned())?,
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
        let recorded: Vec<usize> = called
            .lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| flows.contains(&parameter.binding))
            .map(|(position, _)| position)
            .collect();
        let recorded = TemplateTies::new(&recorded, &[]);
        match self.signature_ties(called) {
            Some(signature) => {
                TemplateTies::widened(Some(signature), Some(recorded)).unwrap_or_default()
            }
            None => recorded,
        }
    }

    fn signature_ties(&self, called: &CalledFunction) -> Option<TemplateTies> {
        let declared = &self.definitions.definition(called.function)?.scheme;
        let Type::Function(kind, _, declared_result) = &declared.body else {
            return None;
        };
        let result = called
            .lambda
            .body
            .tail()
            .map_or(declared_result.as_ref(), |tail| &tail.ty);
        let parameters = called
            .lambda
            .params
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect();
        let moded = Scheme {
            lifetimes: declared.lifetimes.clone(),
            params: declared.params.clone(),
            body: Type::Function(*kind, parameters, Box::new(result.clone())),
        };
        let pipelines = Pipelines::new(self.program.declarations.types().rust_types());
        TemplateTies::of(&moded, pipelines, TemplateAuthor::User)
    }
}
