use crate::types::ty::Type;
use crate::types::unifier::LiteralKind;

pub(in crate::types) trait LiteralUnknowns {
    fn fresh_literal(&mut self, kind: LiteralKind) -> Type;
}
