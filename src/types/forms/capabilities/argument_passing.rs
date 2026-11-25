use crate::diagnostic::Diagnostic;
use crate::types::ty::Type;
use crate::types::typed::{Borrow, TypedExpr};

pub(in crate::types) trait ArgumentPassing {
    fn pass_argument(
        &mut self,
        position: usize,
        parameter: &Type,
        value: &TypedExpr,
    ) -> Result<Borrow, Diagnostic>;
}
