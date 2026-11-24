use rustc_hash::FxHashSet;

use crate::core::BindingId;
use crate::types::{CopyRules, TypedBinder, TypedLambda, TypedProgram};

pub(super) struct LeftOutParameters<'p> {
    program: &'p TypedProgram,
    copy: CopyRules<'p>,
}

impl<'p> LeftOutParameters<'p> {
    pub(super) fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            copy: CopyRules::new(&program.declarations, &program.generics),
        }
    }

    pub(super) fn of(
        &self,
        lambda: &TypedLambda,
        kept: &FxHashSet<BindingId>,
    ) -> FxHashSet<BindingId> {
        lambda
            .params
            .iter()
            .filter(|param| self.may_leave_out(lambda, param))
            .map(|param| param.binding)
            .filter(|binding| !kept.contains(binding))
            .collect()
    }

    fn may_leave_out(&self, lambda: &TypedLambda, param: &TypedBinder) -> bool {
        if self.changes(param) {
            lambda.returns_closure() || self.copy.is_copy(param.ty.referent())
        } else {
            lambda.returns_closure() && self.program.type_holds_borrow(&param.ty)
        }
    }

    fn changes(&self, param: &TypedBinder) -> bool {
        self.program
            .ownership
            .parameter_modes
            .changes(param.binding)
    }
}
