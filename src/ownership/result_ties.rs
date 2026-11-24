use super::tied_parts::TiedParts;
use crate::types::{
    DefinitionTable, ParameterRole, Regions, TemplateBorrows, TemplateTies, Type, TypedExpr,
    TypedLambda, TypedProgram,
};

pub struct ResultTies<'p, 'd> {
    templates: &'p TemplateBorrows,
    definitions: &'d DefinitionTable<'p>,
    regions: &'p Regions,
}

impl<'p, 'd> ResultTies<'p, 'd> {
    pub fn new(program: &'p TypedProgram, definitions: &'d DefinitionTable<'p>) -> Self {
        Self {
            templates: &program.template_borrows,
            definitions,
            regions: &program.ownership.regions,
        }
    }

    pub fn of(&self, callee: &TypedExpr) -> Option<TemplateTies> {
        match self.definitions.called_lambda(callee) {
            Some(lambda) => Some(self.function_ties(lambda)),
            None => self
                .templates
                .read_template(callee, None, |_, ties| ties.cloned()),
        }
    }

    fn function_ties(&self, lambda: &TypedLambda) -> TemplateTies {
        let flows = self.regions.of(ParameterRole::FlowsToResult, lambda);
        let parameters = &lambda.params;
        let mut lent: Vec<usize> = parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| flows.contains(&parameter.binding))
            .map(|(position, _)| position)
            .collect();
        if let Some(result) = lambda.body.tail().map(|tail| &tail.ty) {
            lent.extend(self.signature_lent(lambda, result));
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
        self.regions
            .lending_positions(lambda)
            .into_iter()
            .map(|position| (position, &lambda.params[position].ty))
            .collect()
    }
}
