use super::problems::OwnershipProblems;
use crate::core::BindingId;
use crate::diagnostic::Diagnostics;
use crate::source::Span;
use crate::types::{
    Borrow, ReturnedValues, TypedArgument, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf,
    TypedProgram,
};

pub struct ReturnedChanges<'a> {
    program: &'a TypedProgram,
}

impl<'a> ReturnedChanges<'a> {
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
            if let Some(change) = closures
                .iter()
                .find_map(|closure| Self::change_site(closure, param.binding))
            {
                let bindings = &self.program.bindings;
                diagnostics.push(OwnershipProblems::changed_by_returned_closure(
                    bindings.get(function).name(),
                    bindings.get(param.binding).name(),
                    change,
                ));
            }
        }
    }

    fn change_site(expr: &TypedExpr, param: BindingId) -> Option<Span> {
        if let TypedExprKind::Call(_, arguments) = &expr.kind
            && arguments
                .iter()
                .any(|argument| Self::changes(argument, param))
        {
            return Some(expr.span);
        }
        expr.children()
            .into_iter()
            .find_map(|child| Self::change_site(child, param))
    }

    fn changes(argument: &TypedArgument, param: BindingId) -> bool {
        matches!(argument.value.kind, TypedExprKind::Leaf(TypedLeaf::Local(binding)) if binding == param)
            && matches!(argument.borrow, Borrow::Mutable | Borrow::Reborrow)
    }
}
