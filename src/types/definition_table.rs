use rustc_hash::FxHashMap;

use super::called_function::CalledFunction;
use super::definition_entry::DefinitionEntry;
use super::typed::{
    TypedDefinition, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf, TypedProgram,
};
use crate::core::BindingId;

pub struct DefinitionTable<'p> {
    entries: FxHashMap<BindingId, DefinitionEntry<'p>>,
}

impl<'p> DefinitionTable<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            entries: program
                .modules
                .iter()
                .flat_map(|module| {
                    module.definitions.iter().map(move |definition| {
                        let entry = DefinitionEntry {
                            definition,
                            module: module.id,
                        };
                        (definition.binding, entry)
                    })
                })
                .collect(),
        }
    }

    pub fn entry(&self, binding: BindingId) -> Option<DefinitionEntry<'p>> {
        self.entries.get(&binding).copied()
    }

    pub fn definition(&self, binding: BindingId) -> Option<&'p TypedDefinition> {
        self.entry(binding).map(|entry| entry.definition)
    }

    pub fn called_function<'e>(&self, call: &'e TypedExpr) -> Option<CalledFunction<'p, 'e>> {
        let TypedExprKind::Call(callee, arguments) = &call.kind else {
            return None;
        };
        Some(CalledFunction {
            function: Self::called_global(callee)?,
            lambda: self.called_lambda(callee)?,
            arguments,
        })
    }

    pub fn called_definition(&self, callee: &TypedExpr) -> Option<&'p TypedDefinition> {
        self.definition(Self::called_global(callee)?)
    }

    fn called_lambda(&self, callee: &TypedExpr) -> Option<&'p TypedLambda> {
        self.called_definition(callee)?.value.lambda_value()
    }

    fn called_global(callee: &TypedExpr) -> Option<BindingId> {
        match callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(function)) => Some(function),
            _ => None,
        }
    }
}
