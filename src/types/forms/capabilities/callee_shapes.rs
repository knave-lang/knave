use crate::diagnostic::Diagnostic;
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait CalleeShapes {
    fn function_shape(
        &mut self,
        callee: &TypedExpr,
        argument_count: usize,
    ) -> Result<(Vec<Type>, Type), Diagnostic>;
}
