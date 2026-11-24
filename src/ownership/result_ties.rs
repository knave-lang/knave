use std::cell::RefCell;

use rustc_hash::{FxHashMap, FxHashSet};

use super::tied_parts::TiedParts;
use crate::core::BindingId;
use crate::types::{
    DefinitionTable, Lifetime, ParameterRole, Regions, TemplateTies, Type, TypeLifetimes,
    TypedExpr, TypedLambda, TypedProgram,
};

pub struct ResultTies<'p> {
    program: &'p TypedProgram,
    definitions: DefinitionTable<'p>,
    functions: RefCell<FxHashMap<BindingId, TemplateTies>>,
}

impl<'p> ResultTies<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            definitions: DefinitionTable::of(program),
            functions: RefCell::default(),
        }
    }

    pub fn of(&self, callee: &TypedExpr) -> Option<TemplateTies> {
        let function = DefinitionTable::called_global(callee);
        match function.zip(self.definitions.called_lambda(callee)) {
            Some((function, lambda)) => Some(self.remembered_ties(function, lambda)),
            None => self
                .program
                .template_borrows
                .read_template(callee, None, |_, ties| ties.cloned()),
        }
    }

    fn remembered_ties(&self, function: BindingId, lambda: &TypedLambda) -> TemplateTies {
        self.functions
            .borrow_mut()
            .entry(function)
            .or_insert_with(|| self.function_ties(lambda))
            .clone()
    }

    fn function_ties(&self, lambda: &TypedLambda) -> TemplateTies {
        let Some(result) = lambda.body.tail().map(|tail| &tail.ty) else {
            return TemplateTies::new(&[], &[]);
        };
        let mut lent = self.signature_lent(lambda, result);
        if !Self::names_written_lifetimes(result) {
            lent.extend(self.sharing_positions(lambda, result));
        }
        let tied = TiedParts::of_result(result);
        lent.retain(|position| !tied.ties_only_part_of(&lambda.params[*position].ty));
        TemplateTies::new(&lent, &[])
    }

    fn names_written_lifetimes(result: &Type) -> bool {
        TypeLifetimes::of(result)
            .all()
            .into_iter()
            .any(|lifetime| matches!(lifetime, Lifetime::Written(_)))
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
            let regions = self.regions().of_result(lambda);
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
        self.regions()
            .of_result(lambda)
            .lending_positions()
            .into_iter()
            .map(|position| (position, &lambda.params[position].ty))
            .collect()
    }

    fn regions(&self) -> &'p Regions {
        &self.program.ownership.regions
    }
}
