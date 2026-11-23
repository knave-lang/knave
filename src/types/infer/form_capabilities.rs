use std::rc::Rc;

use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::checker::TypeChecker;
use crate::types::expectation::Expectation;
use crate::types::forms::{ChildTyping, MacroOrigins, TypeFitting};
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

impl ChildTyping for TypeChecker {
    fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
        self.infer(child)
    }
}

impl TypeFitting for TypeChecker {
    fn fit(
        &mut self,
        expectation: Expectation,
        found: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.unification().expect(expectation, found, span)
    }
}

impl MacroOrigins for TypeChecker {
    fn macro_name(&self, span: Span) -> Option<Rc<str>> {
        TypeChecker::macro_name(self, span)
    }
}
