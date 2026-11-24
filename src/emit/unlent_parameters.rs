use super::problems::EmitProblems;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::types::{CopyRules, ParameterCopy, TypedBinder, TypedLambda, TypedProgram};

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
        parameters.unlent = parameters.unlent_parameters();
        parameters
    }

    pub fn is_empty(&self) -> bool {
        self.unlent.is_empty()
    }

    pub fn lends(&self, binding: BindingId) -> bool {
        !self.unlent.contains(&binding)
    }

    pub fn lends_borrow_of(&self, param: &TypedBinder) -> bool {
        self.lends(param.binding) && self.program.type_holds_borrow(&param.ty)
    }

    pub fn uncapturable(&self) -> Option<BindingId> {
        self.lambda
            .params
            .iter()
            .find(|param| {
                !self.lends(param.binding) && self.changes(param) && Self::infers_mut(param)
            })
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

    fn unlent_parameters(&self) -> Vec<BindingId> {
        let candidates: Vec<&TypedBinder> = self
            .lambda
            .params
            .iter()
            .filter(|param| self.may_leave_out(param))
            .collect();
        if candidates.is_empty() {
            return Vec::new();
        }
        let kept = self.program.ownership.regions.kept_by_result(self.lambda);
        candidates
            .into_iter()
            .map(|param| param.binding)
            .filter(|binding| !kept.contains(binding))
            .collect()
    }

    fn may_leave_out(&self, param: &TypedBinder) -> bool {
        if self.changes(param) {
            self.result_cannot_hold(param)
        } else {
            self.lambda.returns_closure() && self.program.type_holds_borrow(&param.ty)
        }
    }

    fn changes(&self, param: &TypedBinder) -> bool {
        self.program
            .ownership
            .parameter_modes
            .changes(param.binding)
    }

    fn result_cannot_hold(&self, param: &TypedBinder) -> bool {
        self.lambda.returns_closure() || self.copy_rules().is_copy(param.ty.referent())
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
