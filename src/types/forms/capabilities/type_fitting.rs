use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::ty::Type;

pub(in crate::types) trait TypeFitting {
    fn fit(&mut self, expectation: Expectation, found: &Type, span: Span)
    -> Result<(), Diagnostic>;
}
