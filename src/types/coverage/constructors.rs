use crate::core::{BindingId, PrimitiveType};
use crate::types::Type;
use crate::types::peeling::ReferencePeeling;
use crate::types::structs::StructTable;
use crate::types::variant_payloads::VariantPayloads;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tag {
    Variant { owner: BindingId, index: usize },
    Struct(BindingId),
    Bool(bool),
}

pub(super) struct Constructor {
    pub tag: Tag,
    pub arguments: Vec<Type>,
}

pub(super) struct ConstructorCatalog<'a> {
    peeling: ReferencePeeling<'a>,
    structs: &'a StructTable,
    payloads: VariantPayloads<'a>,
}

impl<'a> ConstructorCatalog<'a> {
    pub(super) fn new(
        peeling: ReferencePeeling<'a>,
        structs: &'a StructTable,
        payloads: VariantPayloads<'a>,
    ) -> Self {
        Self {
            peeling,
            structs,
            payloads,
        }
    }

    pub(super) fn value_owner(&self, ty: &Type) -> Option<BindingId> {
        let Type::Named(owner, _) = self.peeled(ty) else {
            return None;
        };
        let owned =
            self.payloads.variant_count(owner).is_some() || self.structs.get(owner).is_some();
        owned.then_some(owner)
    }

    pub(super) fn constructors(&self, ty: &Type) -> Option<Vec<Constructor>> {
        match self.peeled(ty) {
            Type::Primitive(PrimitiveType::Bool) => Some(Self::booleans()),
            Type::Named(owner, arguments) => self.named_constructors(owner, &arguments),
            _ => None,
        }
    }

    pub(super) fn peeled(&self, ty: &Type) -> Type {
        self.peeling.peel(ty).0
    }

    fn booleans() -> Vec<Constructor> {
        [false, true]
            .into_iter()
            .map(|value| Constructor {
                tag: Tag::Bool(value),
                arguments: Vec::new(),
            })
            .collect()
    }

    fn named_constructors(&self, owner: BindingId, arguments: &[Type]) -> Option<Vec<Constructor>> {
        match self.payloads.variant_count(owner) {
            Some(count) => Some(self.variant_constructors(owner, count, arguments)),
            None => self
                .struct_constructor(owner, arguments)
                .map(|constructor| vec![constructor]),
        }
    }

    fn variant_constructors(
        &self,
        owner: BindingId,
        count: usize,
        arguments: &[Type],
    ) -> Vec<Constructor> {
        (0..count)
            .map(|index| Constructor {
                tag: Tag::Variant { owner, index },
                arguments: self.payloads.visible(owner, index, arguments),
            })
            .collect()
    }

    fn struct_constructor(&self, owner: BindingId, arguments: &[Type]) -> Option<Constructor> {
        let definition = self.structs.get(owner)?;
        Some(Constructor {
            tag: Tag::Struct(owner),
            arguments: definition
                .instantiated_fields(arguments)
                .into_iter()
                .map(|(_, field_type)| field_type)
                .collect(),
        })
    }
}
