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
        self.types
            .enums
            .get(owner)
            .and_then(|definition| definition.instantiated_payload(index, arguments))
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(payload, stored)| {
                let position = PayloadPosition {
                    variant: index,
                    payload,
                };
                self.seen_by_pattern(owner, position, stored)
            })
            .collect()
    }

    pub(super) fn seen_by_pattern(
        &self,
        owner: BindingId,
        position: PayloadPosition,
        stored: Type,
    ) -> Type {
        match self.is_inferred_box(owner, position) {
            true => self
                .unboxed(stored)
                .expect("an inferred box has one type argument"),
            false => stored,
        }
    }

    pub(super) fn is_inferred_box(&self, owner: BindingId, position: PayloadPosition) -> bool {
        self.types
            .inferred_boxes
            .contains(&BoxedPayload { owner, position })
    }

    fn unboxed(&self, ty: Type) -> Option<Type> {
        match ty {
            Type::Named(binding, arguments)
                if Some(binding) == self.types.box_binding && arguments.len() == 1 =>
            {
                arguments.into_iter().next()
            }
            _ => None,
        }
    }
}
