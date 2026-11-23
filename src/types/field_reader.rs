use std::rc::Rc;

use super::inferred_at::InferredAt;
use super::ty::{GenericId, Type};
use crate::core::BindingId;

pub(super) struct FieldReader {
    pub(super) binding: BindingId,
    pub(super) owner: GenericId,
    pub(super) read: Type,
    pub(super) field: Rc<str>,
    pub(super) trait_binding: BindingId,
    pub(super) inferred_at: InferredAt,
}
