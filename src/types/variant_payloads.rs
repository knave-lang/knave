use super::boxed_payload::BoxedPayload;
use super::payload_position::PayloadPosition;
use super::ty::Type;
use super::type_declarations::TypeDeclarations;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct VariantPayloads<'a> {
    types: &'a TypeDeclarations,
}

impl<'a> VariantPayloads<'a> {
    pub(super) fn new(types: &'a TypeDeclarations) -> Self {
        Self { types }
    }

    pub(super) fn variant_count(&self, owner: BindingId) -> Option<usize> {
        self.types
            .enums
            .get(owner)
            .map(|definition| definition.variants.len())
    }

    pub(super) fn visible(&self, owner: BindingId, index: usize, arguments: &[Type]) -> Vec<Type> {
        let stored = self
            .types
            .enums
            .get(owner)
            .and_then(|definition| definition.instantiated_payload(index, arguments))
            .unwrap_or_default();
        self.seen_by_patterns(owner, index, stored)
    }

    pub(super) fn seen_by_patterns(
        &self,
        owner: BindingId,
        index: usize,
        stored: Vec<Type>,
    ) -> Vec<Type> {
        stored
            .into_iter()
            .enumerate()
            .map(|(payload, stored)| {
                let position = PayloadPosition {
                    variant: index,
                    payload,
                };
                self.seen_by_pattern(BoxedPayload::at(owner, position), stored)
            })
            .collect()
    }

    pub(super) fn seen_by_pattern(&self, payload: BoxedPayload, stored: Type) -> Type {
        match self.is_inferred_box(payload) {
            true => self.unboxed(&stored).clone(),
            false => stored,
        }
    }

    pub(super) fn written<'p>(&self, payload: BoxedPayload, stored: &'p Type) -> &'p Type {
        match self.is_inferred_box(payload) {
            true => self.unboxed(stored),
            false => stored,
        }
    }

    pub(super) fn is_inferred_box(&self, payload: BoxedPayload) -> bool {
        self.types.inferred_boxes.contains(&payload)
    }

    fn unboxed<'p>(&self, inferred_box: &'p Type) -> &'p Type {
        match inferred_box {
            Type::Named(_, arguments) if self.types.is_box(inferred_box) => &arguments[0],
            _ => panic!("an inferred box is a box of one type"),
        }
    }
}
