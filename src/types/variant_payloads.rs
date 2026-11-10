use std::collections::BTreeSet;

use super::enums::EnumTable;
use super::ty::Type;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct VariantPayloads<'a> {
    enums: &'a EnumTable,
    box_binding: Option<BindingId>,
    inferred_boxes: &'a BTreeSet<(BindingId, usize, usize)>,
}

impl<'a> VariantPayloads<'a> {
    pub(super) fn new(
        enums: &'a EnumTable,
        box_binding: Option<BindingId>,
        inferred_boxes: &'a BTreeSet<(BindingId, usize, usize)>,
    ) -> Self {
        Self {
            enums,
            box_binding,
            inferred_boxes,
        }
    }

    pub(super) fn variant_count(&self, owner: BindingId) -> Option<usize> {
        self.enums
            .get(owner)
            .map(|definition| definition.variants.len())
    }

    pub(super) fn visible(&self, owner: BindingId, index: usize, arguments: &[Type]) -> Vec<Type> {
        self.enums
            .get(owner)
            .and_then(|definition| definition.instantiated_payload(index, arguments))
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(payload, ty)| {
                let unboxed = self.unboxed(&ty);
                match self.is_inferred_box(owner, index, payload) {
                    true => unboxed.expect("an inferred box has one type argument"),
                    false => ty,
                }
            })
            .collect()
    }

    pub(super) fn is_inferred_box(&self, owner: BindingId, variant: usize, payload: usize) -> bool {
        self.inferred_boxes.contains(&(owner, variant, payload))
    }

    pub(super) fn unboxed(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Named(binding, arguments)
                if Some(*binding) == self.box_binding && arguments.len() == 1 =>
            {
                Some(arguments[0].clone())
            }
            _ => None,
        }
    }
}
