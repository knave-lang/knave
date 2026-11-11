use std::collections::BTreeSet;

use super::boxed_payload::BoxedPayload;
use super::enums::EnumTable;
use super::rust_types::RustTypeTable;
use super::structs::StructTable;
use crate::core::BindingId;

#[derive(Default)]
pub(super) struct TypeDeclarations {
    pub(super) structs: StructTable,
    pub(super) enums: EnumTable,
    pub(super) rust_types: RustTypeTable,
    pub(super) box_binding: Option<BindingId>,
    pub(super) inferred_boxes: BTreeSet<BoxedPayload>,
}
