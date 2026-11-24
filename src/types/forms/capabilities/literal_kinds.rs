use crate::types::ty::TypeVar;
use crate::types::unifier::LiteralKind;

pub(in crate::types) trait LiteralKinds {
    fn literal_kind(&self, var: TypeVar) -> Option<LiteralKind>;
}
