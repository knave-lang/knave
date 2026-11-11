use std::rc::Rc;

use super::boxed_payload::BoxedPayload;
use super::declared_traits::DeclaredTraits;
use super::selection::Selection;
use super::ty::{Bound, GenericId, Type};
use super::type_declarations::TypeDeclarations;
use super::variant_payloads::VariantPayloads;
use crate::core::BindingId;

pub(super) struct DeriveBlockers<'a> {
    types: &'a TypeDeclarations,
    traits: DeclaredTraits<'a>,
    payloads: VariantPayloads<'a>,
}

impl<'a> DeriveBlockers<'a> {
    pub(super) fn new(
        types: &'a TypeDeclarations,
        traits: DeclaredTraits<'a>,
        payloads: VariantPayloads<'a>,
    ) -> Self {
        Self {
            types,
            traits,
            payloads,
        }
    }

    pub(super) fn blocking_enum_payload(
        &self,
        binding: BindingId,
        trait_binding: BindingId,
    ) -> Option<(Rc<str>, Type)> {
        let definition = self.types.enums.get(binding)?;
        let params: Vec<GenericId> = definition.params.iter().map(|param| param.id).collect();
        definition.blocking_variant(&|position, payload| {
            let boxed = BoxedPayload {
                owner: binding,
                position,
            };
            let written = self.written_payload(boxed, payload, trait_binding);
            self.payload_satisfies(trait_binding, written, &params)
        })
    }

    pub(super) fn blocking_field(
        &self,
        struct_binding: BindingId,
        trait_binding: BindingId,
    ) -> Option<(Rc<str>, Type)> {
        let definition = self.types.structs.get(struct_binding)?;
        let params: Vec<GenericId> = definition.params.iter().map(|param| param.id).collect();
        definition
            .fields
            .iter()
            .find(|(_, ty)| !self.payload_satisfies(trait_binding, ty, &params))
            .map(|(name, ty)| (name.clone(), ty.clone()))
    }

    fn written_payload<'p>(
        &self,
        boxed: BoxedPayload,
        payload: &'p Type,
        trait_binding: BindingId,
    ) -> &'p Type {
        let inferred_box = self.payloads.is_inferred_box(boxed.owner, boxed.position);
        match payload {
            Type::Named(_, arguments)
                if inferred_box && self.traits.is_structural(trait_binding) =>
            {
                &arguments[0]
            }
            _ => payload,
        }
    }

    fn payload_satisfies(
        &self,
        trait_binding: BindingId,
        payload: &Type,
        params: &[GenericId],
    ) -> bool {
        self.field_type_satisfies(&Bound::plain(trait_binding), payload, params)
    }

    fn field_type_satisfies(&self, bound: &Bound, field_type: &Type, params: &[GenericId]) -> bool {
        match field_type {
            Type::Function(..) => self.traits.allows_function_fields(bound.binding),
            Type::Generic(generic) => params.contains(generic),
            _ => match self.traits.select(bound, field_type) {
                Selection::Implemented { requirements, .. } => {
                    requirements.iter().all(|requirement| {
                        self.field_type_satisfies(
                            &requirement.bound,
                            &requirement.implementor,
                            params,
                        )
                    })
                }
                Selection::Undecided | Selection::Missing => false,
            },
        }
    }
}
