use std::cell::RefCell;

use rustc_hash::{FxHashMap, FxHashSet};

use super::tied_parts::TiedParts;
use crate::core::BindingId;
use crate::types::{
    DefinitionTable, ParameterRole, Regions, ResultRegions, TemplateTies, Type, TypeLifetimes,
    TypedDefinition, TypedExpr, TypedLambda, TypedProgram,
};

pub struct ResultTies<'p> {
    program: &'p TypedProgram,
    definitions: DefinitionTable<'p>,
    ties_by_function: RefCell<FxHashMap<BindingId, TemplateTies>>,
}

impl<'p> ResultTies<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            definitions: DefinitionTable::of(program),
            ties_by_function: RefCell::default(),
        }
    }

    pub fn of(&self, callee: &TypedExpr) -> Option<TemplateTies> {
        match self.called_function(callee) {
            Some((definition, lambda)) => Some(self.remembered_ties(definition, lambda)),
            None => self
                .program
                .template_borrows
                .read_template(callee, None, |_, ties| ties.cloned()),
        }
    }

    fn called_function(
        &self,
        callee: &TypedExpr,
    ) -> Option<(&'p TypedDefinition, &'p TypedLambda)> {
        let definition = self.definitions.called_definition(callee)?;
        Some((definition, definition.value.lambda_value()?))
    }

    fn remembered_ties(&self, definition: &TypedDefinition, lambda: &TypedLambda) -> TemplateTies {
        if let Some(known) = self.ties_by_function.borrow().get(&definition.binding) {
            return known.clone();
        }
        let ties = self.function_ties(definition, lambda);
        self.ties_by_function
            .borrow_mut()
            .insert(definition.binding, ties.clone());
        ties
    }

    fn function_ties(&self, definition: &TypedDefinition, lambda: &TypedLambda) -> TemplateTies {
        if let Some(written) = Self::written_result(definition) {
            let lent = self.signature_lent(lambda, written);
            return Self::without_partial_ties(lambda, written, lent);
        }
        let Some(result) = lambda.body.tail().map(|tail| &tail.ty) else {
            return TemplateTies::new(&[], &[]);
        };
        let mut lent = self.signature_lent(lambda, result);
        lent.extend(self.sharing_positions(lambda, result));
        Self::without_partial_ties(lambda, result, lent)
    }

    fn written_result(definition: &TypedDefinition) -> Option<&Type> {
        let Type::Function(_, _, result) = &definition.value.ty else {
            return None;
        };
        TypeLifetimes::of(result)
            .names_written()
            .then_some(result.as_ref())
    }

    fn without_partial_ties(
        lambda: &TypedLambda,
        result: &Type,
        mut lent: Vec<usize>,
    ) -> TemplateTies {
        let tied = TiedParts::of_result(result);
        lent.retain(|position| !tied.ties_only_part_of(&lambda.params[*position].ty));
        TemplateTies::new(&lent, &[])
    }

    fn sharing_positions(&self, lambda: &TypedLambda, result: &Type) -> Vec<usize> {
        let flows = self.regions().of(ParameterRole::FlowsToResult, lambda);
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
            let regions = ResultRegions::of(self.regions(), lambda);
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
        ResultRegions::of(self.regions(), lambda)
            .lending_positions()
            .into_iter()
            .map(|position| (position, &lambda.params[position].ty))
            .collect()
    }

    fn regions(&self) -> &'p Regions {
        &self.program.ownership.regions
    }
}
