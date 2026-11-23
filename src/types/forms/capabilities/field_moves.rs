use crate::types::typed::TypedExpr;

pub(in crate::types) trait FieldMoves {
    fn moved_base(&mut self, base: TypedExpr) -> TypedExpr;
}
