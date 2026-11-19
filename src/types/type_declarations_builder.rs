use super::RustType;
use super::boxes::BoxPlanner;
use super::collected_enum::CollectedEnum;
use super::collected_struct::CollectedStruct;
use super::mismatch_helps::MismatchHelpRow;
use super::type_declarations::TypeDeclarations;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, Diagnostics};

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
        self.types.structs.add(binding, collected.definition);
        diagnostics.extend(collected.problem.into_iter().collect());
    }

    pub(super) fn enter_enum(
        self,
        binding: BindingId,
        collected: CollectedEnum,
        diagnostics: &mut Diagnostics,
    ) {
        if let Some(rust_type) = collected.rust_type {
            self.types.rust_types.add(binding, rust_type);
        }
        self.types.enums.add(binding, collected.definition);
        diagnostics.extend(collected.problems.into_iter().collect());
    }

    pub(super) fn add_rust_type(self, binding: BindingId, rust_type: RustType) {
        self.types.rust_types.add(binding, rust_type);
    }

    pub(super) fn add_mismatch_help(self, row: MismatchHelpRow) -> Result<(), Diagnostic> {
        self.types.mismatch_helps.add(row)
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
