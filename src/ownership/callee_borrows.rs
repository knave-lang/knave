use super::function_memo::FunctionMemo;
use super::result_ties::ResultTies;
use super::store_pairs::StorePairs;
use super::store_ties::StoreTies;
use crate::types::{DefinitionTable, TemplateTies, TypedExpr, TypedProgram};

pub struct CalleeBorrows<'p> {
    program: &'p TypedProgram,
    definitions: DefinitionTable<'p>,
    result_ties: ResultTies<'p>,
    store_ties: StoreTies<'p>,
    remembered_ties: FunctionMemo<TemplateTies>,
    remembered_stores: FunctionMemo<StorePairs>,
}

impl<'p> CalleeBorrows<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            definitions: DefinitionTable::of(program),
            result_ties: ResultTies::new(program),
            store_ties: StoreTies::new(program),
            remembered_ties: FunctionMemo::default(),
            remembered_stores: FunctionMemo::default(),
        }
    }

    pub fn result_ties(&self, callee: &TypedExpr) -> Option<TemplateTies> {
        match self.definitions.called_lambda(callee) {
            Some((definition, lambda)) => {
                Some(self.remembered_ties.remembered(definition.binding, || {
                    self.result_ties.of_function(definition, lambda)
                }))
            }
            None => self
                .program
                .template_borrows
                .read_template(callee, None, |_, ties| ties.cloned()),
        }
    }

    pub fn stores(&self, callee: &TypedExpr) -> StorePairs {
        let Some((definition, lambda)) = self.definitions.called_lambda(callee) else {
            return StorePairs::default();
        };
        self.remembered_stores.remembered(definition.binding, || {
            self.store_ties.of_function(definition, lambda)
        })
    }
}
