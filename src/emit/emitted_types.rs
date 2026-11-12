use super::unsized_parameters::UnsizedParameters;
use crate::core::BindingId;
use crate::types::{
    BoxedPayload, EnumTable, GenericId, RustTypeTable, StructTable, TypeDeclarations,
};

#[derive(Clone, Copy)]
pub struct EmittedTypes<'a> {
    declared: &'a TypeDeclarations,
    unsized_parameters: &'a UnsizedParameters,
}

impl<'a> EmittedTypes<'a> {
    pub fn new(declared: &'a TypeDeclarations, unsized_parameters: &'a UnsizedParameters) -> Self {
        Self {
            declared,
            unsized_parameters,
        }
    }

    pub fn structs(&self) -> &'a StructTable {
        self.declared.structs()
    }

    pub fn enums(&self) -> &'a EnumTable {
        self.declared.enums()
    }

    pub fn rust_types(&self) -> &'a RustTypeTable {
        self.declared.rust_types()
    }

    pub fn box_binding(&self) -> Option<BindingId> {
        self.declared.box_binding()
    }

    pub fn boxes(&self, payload: &BoxedPayload) -> bool {
        self.declared.boxes(payload)
    }

    pub fn is_unsized(&self, generic: GenericId) -> bool {
        self.unsized_parameters.contains(generic)
    }
}
