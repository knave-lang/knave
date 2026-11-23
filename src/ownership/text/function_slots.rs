use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::text_position::{TextPosition, TextPositions};
use super::text_value::TextValue;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::types::Type;

pub struct FunctionSlots<'t> {
    parameters: Vec<(SlotOwner, &'t Type)>,
    result: (SlotOwner, &'t Type),
}

impl<'t> FunctionSlots<'t> {
    pub fn new(parameters: Vec<(SlotOwner, &'t Type)>, result: (SlotOwner, &'t Type)) -> Self {
        Self { parameters, result }
    }

    pub fn parameter_path(part: usize) -> TypePath {
        TypePath::default().child(part)
    }

    pub fn result_path() -> TypePath {
        TypePath::default().result()
    }

    pub fn positions(&self) -> Vec<(TypePath, Slot, TextPosition)> {
        let parameters = self
            .parameters
            .iter()
            .enumerate()
            .map(|(part, (owner, ty))| (Self::parameter_path(part), owner, *ty));
        let (owner, ty) = &self.result;
        parameters
            .chain([(Self::result_path(), owner, *ty)])
            .flat_map(|(base, owner, ty)| {
                TextPositions::of(ty).into_iter().map(move |position| {
                    let slot = Slot::new(owner.clone(), position.path.clone());
                    (base.joined(&position.path), slot, position)
                })
            })
            .collect()
    }

    pub fn value(&self, context: &mut WalkContext<'_, '_>) -> TextValue {
        self.positions()
            .into_iter()
            .fold(TextValue::default(), |value, (path, slot, _)| {
                value.nesting(path, context.node(slot))
            })
    }
}
