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
            let boxed = BoxedPayload::at(binding, position);
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
        match self.traits.is_structural(trait_binding) {
            true => self.payloads.written(boxed, payload),
            false => payload,
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

#[cfg(test)]
mod tests {
    use crate::core::PrimitiveType;
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;

    const SOURCE: &str = "
        (struct ticket (id u32) (owner string))
        (struct (holder a) (value a))
        (struct locker (opener (fn () i64)))
        (struct shelf (labels (vec string)))
        (struct toolbox (openers (vec (fn () i64))))
        (enum reading (text string) (number f64))
        (rust-type octal-out \"std::fmt::Formatter<'_>\")
        (rust-type octal-result \"std::fmt::Result\")
        (rust-trait octal \"std::fmt::Octal\"
          (method fmt
            (fn ((ref self) (mut octal-out))
              octal-result))
          (structural
            :struct \"write!({1}, \\\"{name}\\\")\"
            :field \"\"
            :variant \"write!({1}, \\\"{name}\\\")\"
            :part \"\"
            :bare \"write!({1}, \\\"{name}\\\")\")
          u32)
        (enum chain
          end
          (link u32 chain))
        (enum (slot a) (filled a) empty)
        (defn main () (do))";

    struct BlockerFixture {
        declarations: DeclarationsFixture,
    }

    impl BlockerFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn blocking_field(&self, owner: &str, trait_name: &str) -> Option<String> {
            let owner = self.declarations.type_named(owner);
            let trait_binding = self.declarations.trait_named(trait_name);
            self.declarations
                .derive_blockers()
                .blocking_field(owner, trait_binding)
                .map(|(field, _)| field.to_string())
        }

        fn blocking_payload(&self, owner: &str, trait_name: &str) -> Option<(String, Type)> {
            let owner = self.declarations.type_named(owner);
            let trait_binding = self.declarations.trait_named(trait_name);
            self.declarations
                .derive_blockers()
                .blocking_enum_payload(owner, trait_binding)
                .map(|(variant, payload)| (variant.to_string(), payload))
        }
    }

    #[test]
    fn a_field_without_the_trait_blocks_it() {
        let fixture = BlockerFixture::new();
        assert_eq!(
            fixture.blocking_field("ticket", "copy"),
            Some("owner".to_string())
        );
    }

    #[test]
    fn fields_that_all_have_the_trait_block_nothing() {
        let fixture = BlockerFixture::new();
        assert_eq!(fixture.blocking_field("ticket", "clone"), None);
    }

    #[test]
    fn a_field_of_a_type_parameter_blocks_nothing() {
        let fixture = BlockerFixture::new();
        assert_eq!(fixture.blocking_field("holder", "copy"), None);
    }

    #[test]
    fn a_function_field_blocks_equality() {
        let fixture = BlockerFixture::new();
        assert_eq!(
            fixture.blocking_field("locker", "eq"),
            Some("opener".to_string())
        );
    }

    #[test]
    fn a_function_field_does_not_block_debug() {
        let fixture = BlockerFixture::new();
        assert_eq!(fixture.blocking_field("locker", "debug"), None);
    }

    #[test]
    fn a_collection_whose_items_have_the_trait_blocks_nothing() {
        let fixture = BlockerFixture::new();
        assert_eq!(fixture.blocking_field("shelf", "clone"), None);
    }

    #[test]
    fn a_collection_whose_items_lack_the_trait_blocks_it() {
        let fixture = BlockerFixture::new();
        assert_eq!(
            fixture.blocking_field("toolbox", "eq"),
            Some("openers".to_string())
        );
    }

    #[test]
    fn a_variant_payload_without_the_trait_blocks_it() {
        let fixture = BlockerFixture::new();
        assert_eq!(
            fixture.blocking_payload("reading", "eq"),
            Some(("number".to_string(), Type::Primitive(PrimitiveType::F64)))
        );
    }

    #[test]
    fn a_payload_of_a_type_parameter_blocks_nothing() {
        let fixture = BlockerFixture::new();
        assert_eq!(fixture.blocking_payload("slot", "copy"), None);
    }

    #[test]
    fn an_inferred_box_blocks_a_trait_that_is_not_structural() {
        let fixture = BlockerFixture::new();
        let chain = fixture.declarations.type_named("chain");
        let (variant, payload) = fixture
            .blocking_payload("chain", "copy")
            .expect("a box is never copy");
        assert_eq!(variant, "link");
        assert!(fixture.declarations.declared_types().is_box(&payload));
        assert_eq!(payload.parts(), vec![&Type::Named(chain, Vec::new())]);
    }

    #[test]
    fn an_inferred_box_is_seen_through_for_a_structural_trait() {
        let fixture = BlockerFixture::new();
        assert_eq!(fixture.blocking_payload("chain", "octal"), None);
    }
}
