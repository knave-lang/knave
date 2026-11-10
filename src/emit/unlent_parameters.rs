use super::held_borrows::HeldBorrows;
use super::problems::EmitProblems;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::types::{CopyRules, ParameterCopy, Type, TypedBinder, TypedLambda, TypedProgram};
use rustc_hash::FxHashSet;

pub struct UnlentParameters<'a> {
    program: &'a TypedProgram,
    lambda: &'a TypedLambda,
    unlent: Vec<BindingId>,
}

impl<'a> UnlentParameters<'a> {
    pub fn of(
        program: &'a TypedProgram,
        lambda: &'a TypedLambda,
        shared_flows: impl FnOnce() -> FxHashSet<BindingId>,
    ) -> Self {
        let mut parameters = Self {
            program,
            lambda,
            unlent: Vec::new(),
        };
        parameters.unlent = parameters.changed_parameters(shared_flows);
        parameters
    }

    pub fn is_empty(&self) -> bool {
        self.unlent.is_empty()
    }

    pub fn lends(&self, binding: BindingId) -> bool {
        !self.unlent.contains(&binding)
    }

    pub fn uncapturable(&self) -> Option<BindingId> {
        self.lambda
            .params
            .iter()
            .find(|param| !self.lends(param.binding) && Self::infers_mut(param))
            .map(|param| param.binding)
    }

    pub fn uncapturable_problem(&self, binding: BindingId) -> Diagnostic {
        let param = self.parameter(binding);
        let name = self.program.bindings.get(binding).name();
        let copy = ParameterCopy::of(name, param, &self.copy_rules());
        EmitProblems::uncapturable_changed_parameter(copy, param.span)
    }

    fn infers_mut(param: &TypedBinder) -> bool {
        !param.annotated
    }

    fn changed_parameters(
        &self,
        shared_flows: impl FnOnce() -> FxHashSet<BindingId>,
    ) -> Vec<BindingId> {
        let changed: Vec<&TypedBinder> = self
            .lambda
            .params
            .iter()
            .filter(|param| self.program.parameter_modes.changes(param.binding))
            .filter(|param| self.result_cannot_hold(param))
            .collect();
        if changed.is_empty() || HeldBorrows::new(self.program).in_returned_closures(self.lambda) {
            return Vec::new();
        }
        let flowing = shared_flows();
        changed
            .into_iter()
            .map(|param| param.binding)
            .filter(|binding| !flowing.contains(binding))
            .collect()
    }

    fn result_cannot_hold(&self, param: &TypedBinder) -> bool {
        let returns_closure = self
            .lambda
            .body
            .tail()
            .is_some_and(|tail| matches!(tail.ty, Type::Function(..)));
        returns_closure || self.copy_rules().is_copy(param.ty.referent())
    }

    fn copy_rules(&self) -> CopyRules<'_> {
        CopyRules::new(
            &self.program.traits,
            &self.program.generics,
            &self.program.structs,
            &self.program.enums,
        )
    }

    fn parameter(&self, binding: BindingId) -> &TypedBinder {
        self.lambda
            .params
            .iter()
            .find(|param| param.binding == binding)
            .expect("the binding names a parameter")
    }
}
