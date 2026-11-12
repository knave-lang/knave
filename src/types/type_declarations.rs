use std::collections::BTreeSet;

use super::boxed_payload::BoxedPayload;
use super::enums::EnumTable;
use super::rust_types::RustTypeTable;
use super::structs::StructTable;
use super::ty::Type;
use crate::core::BindingId;

#[derive(Default)]
pub struct TypeDeclarations {
    pub(super) structs: StructTable,
    pub(super) enums: EnumTable,
    pub(super) rust_types: RustTypeTable,
    pub(super) box_binding: Option<BindingId>,
    pub(super) inferred_boxes: BTreeSet<BoxedPayload>,
}

impl TypeDeclarations {
    pub fn structs(&self) -> &StructTable {
        &self.structs
    }

    pub fn enums(&self) -> &EnumTable {
        &self.enums
    }

    pub fn rust_types(&self) -> &RustTypeTable {
        &self.rust_types
    }

    pub fn box_binding(&self) -> Option<BindingId> {
        self.box_binding
    }

    pub fn boxes(&self, payload: &BoxedPayload) -> bool {
        self.inferred_boxes.contains(payload)
    }

    pub fn is_box(&self, candidate: &Type) -> bool {
        match candidate {
            Type::Named(binding, arguments) => {
                Some(*binding) == self.box_binding && arguments.len() == 1
            }
            _ => false,
        }
    }

    pub fn boxed_object(&self, candidate: &Type) -> Option<BindingId> {
        match candidate {
            Type::Named(_, arguments) if self.is_box(candidate) => match arguments.as_slice() {
                [Type::TraitObject(object)] => Some(*object),
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::boxed_payload::BoxedPayload;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::{PayloadPosition, Type};

    struct BoxFixture {
        declarations: DeclarationsFixture,
    }

    impl BoxFixture {
        const SOURCE: &str = "
            (enum chain end (link u32 chain))
            (trait drawn (draw (fn ((ref self)) i64)))
            (defn main () (do))";
        const LINK: usize = 1;
        const LINK_NUMBER: usize = 0;
        const LINK_REST: usize = 1;

        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(Self::SOURCE),
            }
        }

        fn boxed(&self, payloads: Vec<Type>) -> Type {
            let box_binding = self
                .declarations
                .types()
                .box_binding()
                .expect("the prelude declares a box");
            Type::Named(box_binding, payloads)
        }

        fn object(&self, trait_name: &str) -> Type {
            Type::TraitObject(self.declarations.trait_named(trait_name))
        }

        fn link_payload(&self, payload: usize) -> BoxedPayload {
            let chain = self.declarations.type_named("chain");
            BoxedPayload::at(
                chain,
                PayloadPosition {
                    variant: Self::LINK,
                    payload,
                },
            )
        }
    }

    #[test]
    fn a_box_of_one_payload_is_a_box() {
        let fixture = BoxFixture::new();
        let boxed = fixture.boxed(vec![fixture.object("drawn")]);
        assert!(fixture.declarations.types().is_box(&boxed));
    }

    #[test]
    fn a_box_without_a_payload_is_not_a_box() {
        let fixture = BoxFixture::new();
        assert!(
            !fixture
                .declarations
                .types()
                .is_box(&fixture.boxed(Vec::new()))
        );
    }

    #[test]
    fn a_box_of_two_payloads_is_not_a_box() {
        let fixture = BoxFixture::new();
        let object = fixture.object("drawn");
        let boxed = fixture.boxed(vec![object.clone(), object]);
        assert!(!fixture.declarations.types().is_box(&boxed));
    }

    #[test]
    fn a_box_of_a_trait_object_holds_that_trait() {
        let fixture = BoxFixture::new();
        let boxed = fixture.boxed(vec![fixture.object("drawn")]);
        let drawn = fixture.declarations.trait_named("drawn");
        assert_eq!(
            fixture.declarations.types().boxed_object(&boxed),
            Some(drawn)
        );
    }

    #[test]
    fn a_box_of_a_plain_type_holds_no_trait_object() {
        let fixture = BoxFixture::new();
        let chain = Type::Named(fixture.declarations.type_named("chain"), Vec::new());
        let boxed = fixture.boxed(vec![chain]);
        assert_eq!(fixture.declarations.types().boxed_object(&boxed), None);
    }

    #[test]
    fn a_trait_object_outside_a_box_is_not_boxed() {
        let fixture = BoxFixture::new();
        let object = fixture.object("drawn");
        assert_eq!(fixture.declarations.types().boxed_object(&object), None);
    }

    #[test]
    fn a_trait_object_in_another_named_type_is_not_boxed() {
        let fixture = BoxFixture::new();
        let chain = fixture.declarations.type_named("chain");
        let wrapped = Type::Named(chain, vec![fixture.object("drawn")]);
        assert_eq!(fixture.declarations.types().boxed_object(&wrapped), None);
    }

    #[test]
    fn a_recursive_payload_is_an_inferred_box() {
        let fixture = BoxFixture::new();
        let rest = fixture.link_payload(BoxFixture::LINK_REST);
        assert!(fixture.declarations.types().boxes(&rest));
    }

    #[test]
    fn a_plain_payload_is_not_an_inferred_box() {
        let fixture = BoxFixture::new();
        let number = fixture.link_payload(BoxFixture::LINK_NUMBER);
        assert!(!fixture.declarations.types().boxes(&number));
    }
}
