use super::loan_names::LoanNames;
use crate::source::Span;
use crate::types::TypedLambda;

#[derive(Clone, Copy)]
pub struct Closure<'w> {
    lambda: &'w TypedLambda,
    span: Span,
}

impl<'w> Closure<'w> {
    pub fn new(lambda: &'w TypedLambda, span: Span) -> Self {
        Self { lambda, span }
    }

    pub fn lambda(&self) -> &'w TypedLambda {
        self.lambda
    }

    pub fn heading(&self, names: &LoanNames<'_>) -> String {
        format!("closure at {}", names.position(self.span))
    }
}
