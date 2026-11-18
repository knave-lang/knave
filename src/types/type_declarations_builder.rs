use super::RustType;
use super::boxes::BoxPlanner;
use super::collected_enum::CollectedEnum;
use super::collected_struct::CollectedStruct;
use super::type_declarations::TypeDeclarations;
use crate::core::BindingId;
use crate::diagnostic::Diagnostics;

pub(super) struct TypeDeclarationsBuilder<'a> {
    types: &'a mut TypeDeclarations,
}

impl<'a> TypeDeclarationsBuilder<'a> {
    pub(super) fn new(types: &'a mut TypeDeclarations) -> Self {
        Self { types }
    }

    pub(super) fn enter_struct(
        self,
        binding: BindingId,
        collected: CollectedStruct,
        diagnostics: &mut Diagnostics,
    ) {
        collected.enter(binding, self.types, diagnostics);
    }

    pub(super) fn enter_enum(
        self,
        binding: BindingId,
        collected: CollectedEnum,
        diagnostics: &mut Diagnostics,
    ) {
        collected.enter(binding, self.types, diagnostics);
    }

    pub(super) fn add_rust_type(self, binding: BindingId, rust_type: RustType) {
        self.types.rust_types.add(binding, rust_type);
    }

    pub(super) fn plan_boxes(self) {
        let types = self.types;
        types.box_binding = types.rust_types.box_binding();
        if let Some(box_binding) = types.box_binding {
            types.inferred_boxes =
                BoxPlanner::add_boxes(&mut types.structs, &mut types.enums, box_binding);
        }
    }
}
