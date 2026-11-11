use super::rust_types::RustTypeTable;
use super::structs::StructTable;
use super::ty::Type;
use super::view_coercion::ViewCoercion;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct Declarations<'a> {
    structs: &'a StructTable,
    rust_types: &'a RustTypeTable,
}

impl<'a> Declarations<'a> {
    pub(super) fn new(structs: &'a StructTable, rust_types: &'a RustTypeTable) -> Self {
        Self {
            structs,
            rust_types,
        }
    }

    pub(super) fn field_type(
        &self,
        owner: BindingId,
        label: &str,
        arguments: &[Type],
    ) -> Option<Type> {
        self.structs
            .get(owner)
            .and_then(|definition| definition.instantiated_field(label, arguments))
    }

    pub(super) fn viewed_items(&self, coercion: ViewCoercion) -> Option<Vec<(Type, Type)>> {
        coercion.viewed_items(self.rust_types)
    }
}
