use crate::types::positional_read::{PairPart, PositionalRead};
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait PairParts {
    fn pair_part(&self, read: &PositionalRead, pair: &Type) -> Option<PairPart>;
    fn read_part(&self, part: PairPart, pair: TypedExpr) -> TypedExpr;
}
