use rustc_hash::FxHashSet;

use super::tied_parts::TiedParts;
use crate::core::BindingId;
use crate::types::{
    DefinitionTable, ParameterRole, Regions, TemplateBorrows, TemplateTies, Type, TypedExpr,
    TypedLambda, TypedProgram,
};

pub struct ResultTies<'p, 'd> {
    program: &'p TypedProgram,
    templates: &'p TemplateBorrows,
    definitions: &'d DefinitionTable<'p>,
    regions: &'p Regions,
}

impl<'p, 'd> ResultTies<'p, 'd> {
    pub fn new(program: &'p TypedProgram, definitions: &'d DefinitionTable<'p>) -> Self {
        Self {
            program,
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
        let Some(result) = lambda.body.tail().map(|tail| &tail.ty) else {
            return TemplateTies::new(&[], &[]);
        };
        let mut lent = self.sharing_positions(lambda, result);
        lent.extend(self.signature_lent(lambda, result));
        let tied = TiedParts::of_result(result);
        lent.retain(|position| !tied.ties_only_part_of(&lambda.params[*position].ty));
        TemplateTies::new(&lent, &[])
    }

    fn sharing_positions(&self, lambda: &TypedLambda, result: &Type) -> Vec<usize> {
        let flows = self.regions.of(ParameterRole::FlowsToResult, lambda);
        let sharing = self.sharing_while_borrowing(lambda, result);
        lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| {
                flows.contains(&parameter.binding)
                    || (sharing.contains(&parameter.binding)
                        && self.program.type_holds_borrow(&parameter.ty))
            })
            .map(|(position, _)| position)
            .collect()
    }

    fn sharing_while_borrowing(&self, lambda: &TypedLambda, result: &Type) -> FxHashSet<BindingId> {
        if self.program.type_holds_borrow(result) {
            let regions = self.regions.of_result(lambda);
            let store_only = regions.store_only_targets();
            let mut sharing = regions.sharing();
            sharing.retain(|binding| !store_only.contains(binding));
            sharing
        } else {
            FxHashSet::default()
        }
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
            .of_result(lambda)
            .lending_positions()
            .into_iter()
            .map(|position| (position, &lambda.params[position].ty))
            .collect()
    }
}
