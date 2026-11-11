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

#[cfg(test)]
impl TypeDeclarations {
    pub(super) fn taken_from(program: &mut super::TypedProgram) -> Self {
        Self {
            structs: std::mem::take(&mut program.structs),
            enums: std::mem::take(&mut program.enums),
            rust_types: std::mem::take(&mut program.rust_types),
            box_binding: program.box_binding,
            inferred_boxes: std::mem::take(&mut program.inferred_boxes),
        }
    }
}
