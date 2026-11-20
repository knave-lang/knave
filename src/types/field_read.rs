use super::read_kind::ReadKind;
use super::ty::Type;

#[derive(Clone, PartialEq)]
pub struct FieldRead {
    pub read: Type,
    pub kind: ReadKind,
}
