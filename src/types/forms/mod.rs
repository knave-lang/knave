mod conditional;

use std::rc::Rc;

pub(super) use conditional::{Conditional, IfBranches};

use super::expectation::Expectation;
use super::ty::Type;
use super::typed::TypedExpr;
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) trait ExpressionForm<C> {
    fn type_against(
        &self,
        context: &mut C,
        expectation: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic>;
}

pub(super) trait ChildTyping {
    fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic>;
}

pub(super) trait TypeFitting {
    fn fit(&mut self, expectation: Expectation, found: &Type, span: Span)
    -> Result<(), Diagnostic>;
}

pub(super) trait MacroOrigins {
    fn macro_name(&self, span: Span) -> Option<Rc<str>>;
}
