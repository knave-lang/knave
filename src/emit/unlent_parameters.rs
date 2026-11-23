use super::problems::EmitProblems;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::types::{
    CopyRules, ParameterCopy, ParameterRole, Type, TypedBinder, TypedLambda, TypedProgram,
};

pub struct UnlentParameters<'a> {
    program: &'a TypedProgram,
    lambda: &'a TypedLambda,
    unlent: Vec<BindingId>,
}

impl<'a> UnlentParameters<'a> {
    pub fn of(program: &'a TypedProgram, lambda: &'a TypedLambda) -> Self {
        let mut parameters = Self {
            program,
            lambda,
            unlent: Vec::new(),
        };
        parameters.unlent = parameters.changed_parameters();
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

    fn changed_parameters(&self) -> Vec<BindingId> {
        let changed: Vec<&TypedBinder> = self
            .lambda
            .params
            .iter()
            .filter(|param| {
                self.program
                    .ownership
                    .parameter_modes
                    .changes(param.binding)
            })
            .filter(|param| self.result_cannot_hold(param))
            .collect();
        if changed.is_empty() {
            return Vec::new();
        }
        let regions = &self.program.ownership.regions;
        let held = regions.of(ParameterRole::HeldByResult, self.lambda);
        let flowing = regions.of(ParameterRole::FlowsThroughSharedArguments, self.lambda);
        changed
            .into_iter()
            .map(|param| param.binding)
            .filter(|binding| !held.contains(binding) && !flowing.contains(binding))
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
        CopyRules::new(&self.program.declarations, &self.program.generics)
    }

    fn parameter(&self, binding: BindingId) -> &TypedBinder {
        self.lambda
            .params
            .iter()
            .find(|param| param.binding == binding)
            .expect("the binding names a parameter")
    }
}
