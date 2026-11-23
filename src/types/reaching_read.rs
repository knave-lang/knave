use super::field_read::FieldRead;
use super::ty::Type;
use crate::core::BindingId;

pub(super) struct ReachingRead {
    pub(super) owner: BindingId,
    pub(super) arguments: Vec<Type>,
    pub(super) read: FieldRead,
}
