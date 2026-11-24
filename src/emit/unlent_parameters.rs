use rustc_hash::FxHashSet;

use super::problems::EmitProblems;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::types::{
    CopyRules, ParameterCopy, ParameterRole, TypedBinder, TypedLambda, TypedProgram,
};

pub struct UnlentParameters<'a> {
    program: &'a TypedProgram,
    lambda: &'a TypedLambda,
    unlent: FxHashSet<BindingId>,
}

impl<'a> UnlentParameters<'a> {
    pub fn of(program: &'a TypedProgram, lambda: &'a TypedLambda) -> Self {
        Self {
            program,
            lambda,
            unlent: program
                .ownership
                .regions
                .of(ParameterRole::LeftOutOfResult, lambda),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.unlent.is_empty()
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

    fn changes(&self, param: &TypedBinder) -> bool {
        self.program
            .ownership
            .parameter_modes
            .changes(param.binding)
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

    fn lends(&self, binding: BindingId) -> bool {
        !self.unlent.contains(&binding)
    }
}
