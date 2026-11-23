use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait PlaceReads {
    fn read_type(&self, value: &TypedExpr) -> Type;
    fn changed_type(&self, value: &TypedExpr) -> Type;
    fn field_place(&self, value: &TypedExpr) -> Option<String>;
}
