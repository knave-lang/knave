use super::tied_parts::TiedParts;
use crate::types::{
    DefinitionTable, ParameterRole, Regions, TemplateBorrows, TemplateTies, Type, TypedExpr,
    TypedExprKind, TypedLambda, TypedLeaf, TypedProgram,
};

pub struct RecordedTies<'p, 'd> {
    templates: &'p TemplateBorrows,
    definitions: &'d DefinitionTable<'p>,
    regions: &'d Regions,
}

impl<'p, 'd> RecordedTies<'p, 'd> {
    pub fn new(
        program: &'p TypedProgram,
        definitions: &'d DefinitionTable<'p>,
        regions: &'d Regions,
    ) -> Self {
        Self {
            templates: &program.template_borrows,
            definitions,
            regions,
        }
    }

    pub fn of(&self, callee: &TypedExpr) -> Option<TemplateTies> {
        match self.called_lambda(callee) {
            Some(lambda) => Some(self.function_ties(lambda)),
            None => self
                .templates
                .read_template(callee, None, |_, ties| ties.cloned()),
        }
    }

    fn called_lambda(&self, callee: &TypedExpr) -> Option<&'p TypedLambda> {
        let TypedExprKind::Leaf(TypedLeaf::Global(function)) = callee.kind else {
            return None;
        };
        self.definitions.definition(function)?.value.lambda_value()
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
