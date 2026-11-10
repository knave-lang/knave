use super::problems::OwnershipProblems;
use crate::core::BindingId;
use crate::diagnostic::Diagnostics;
use crate::source::Span;
use crate::types::{
    ReturnedValues, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf, TypedProgram,
};

pub struct ReturnedClosureUses<'a> {
    program: &'a TypedProgram,
}

impl<'a> ReturnedClosureUses<'a> {
    pub fn new(program: &'a TypedProgram) -> Self {
        Self { program }
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
        for param in lambda.params.iter().filter(|param| {
            self.program
                .changed_copy_parameters
                .contains(&param.binding)
        }) {
            if let Some(usage) = closures
                .iter()
                .find_map(|closure| Self::use_site(closure, param.binding))
            {
                let bindings = &self.program.bindings;
                diagnostics.push(OwnershipProblems::changed_parameter_in_returned_closure(
                    bindings.get(function).name(),
                    bindings.get(param.binding).name(),
                    usage,
                ));
            }
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
