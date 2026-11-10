use super::problems::OwnershipProblems;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::Span;
use crate::types::{
    CopyRules, ReturnedValues, TypedBinder, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf,
    TypedProgram,
};

pub struct ReturnedClosureUses<'a> {
    program: &'a TypedProgram,
    copy: &'a CopyRules<'a>,
}

impl<'a> ReturnedClosureUses<'a> {
    pub fn new(program: &'a TypedProgram, copy: &'a CopyRules<'a>) -> Self {
        Self { program, copy }
    }

    pub fn check(&self) -> Diagnostics {
        let mut diagnostics = Diagnostics::default();
        for definition in self
            .program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
        {
            if let TypedExprKind::Fn(lambda) = &definition.value.kind {
                self.check_function(definition.binding, lambda, &mut diagnostics);
            }
        }
        diagnostics
    }

    fn check_function(
        &self,
        function: BindingId,
        lambda: &TypedLambda,
        diagnostics: &mut Diagnostics,
    ) {
        let closures = ReturnedValues::new(self.program).closures(lambda);
        for param in lambda
            .params
            .iter()
            .filter(|param| self.program.changed_parameters.contains(&param.binding))
        {
            if let Some(usage) = closures
                .iter()
                .find_map(|closure| Self::use_site(closure, param.binding))
            {
                diagnostics.push(self.problem(function, param, usage));
            }
        }
    }

    fn problem(&self, function: BindingId, param: &TypedBinder, usage: Span) -> Diagnostic {
        let function = self.program.bindings.get(function).name();
        let name = self.program.bindings.get(param.binding).name();
        if self.copy.is_copy(param.ty.referent()) {
            OwnershipProblems::changed_copy_parameter_in_returned_closure(function, name, usage)
        } else {
            OwnershipProblems::changed_owned_parameter_in_returned_closure(function, name, usage)
        }
    }

    fn use_site(expr: &TypedExpr, param: BindingId) -> Option<Span> {
        if matches!(expr.kind, TypedExprKind::Leaf(TypedLeaf::Local(binding)) if binding == param) {
            return Some(expr.span);
        }
        expr.children()
            .into_iter()
            .find_map(|child| Self::use_site(child, param))
    }
}
