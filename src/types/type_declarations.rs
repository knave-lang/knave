use std::collections::BTreeSet;

use super::boxed_payload::BoxedPayload;
use super::enums::EnumTable;
use super::rust_types::RustTypeTable;
use super::structs::StructTable;
use super::ty::Type;
use crate::core::BindingId;

#[derive(Default)]
pub struct TypeDeclarations {
    pub(super) structs: StructTable,
    pub(super) enums: EnumTable,
    pub(super) rust_types: RustTypeTable,
    pub(super) box_binding: Option<BindingId>,
    pub(super) inferred_boxes: BTreeSet<BoxedPayload>,
}

impl TypeDeclarations {
    pub fn structs(&self) -> &StructTable {
        &self.structs
    }

    pub fn enums(&self) -> &EnumTable {
        &self.enums
    }

    pub fn rust_types(&self) -> &RustTypeTable {
        &self.rust_types
    }

    pub fn box_binding(&self) -> Option<BindingId> {
        self.box_binding
    }

    pub fn boxes(&self, payload: &BoxedPayload) -> bool {
        self.inferred_boxes.contains(payload)
    }

    pub fn is_box(&self, candidate: &Type) -> bool {
        match candidate {
            Type::Named(binding, arguments) => {
                Some(*binding) == self.box_binding && arguments.len() == 1
            }
            _ => false,
        }
    }

    pub fn is_boxed_trait(&self, implementor: &Type, trait_binding: BindingId) -> bool {
        let Some(box_binding) = self.box_binding else {
            return false;
        };
        matches!(
            implementor,
            Type::Named(binding, arguments)
                if *binding == box_binding
                    && matches!(arguments.as_slice(), [Type::TraitObject(object)] if *object == trait_binding)
        )
    }
}
