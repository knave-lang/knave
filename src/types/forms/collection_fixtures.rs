use crate::core::{BindingId, PrimitiveType};
use crate::types::collection::Collection;
use crate::types::rust_types::{RustType, RustTypeTable};
use crate::types::ty::Type;

pub(in crate::types) struct CollectionFixtures {
    rust_types: RustTypeTable,
}

impl CollectionFixtures {
    const MAP: BindingId = BindingId::from_index(1);
    const VECTOR: BindingId = BindingId::from_index(2);
    const SLICE: BindingId = BindingId::from_index(3);

    pub(in crate::types) fn new() -> Self {
        let mut rust_types = RustTypeTable::default();
        rust_types.add(Self::MAP, RustType::hash_map());
        rust_types.add(Self::VECTOR, RustType::vector());
        Self { rust_types }
    }

    pub(in crate::types) fn collection_of(&self, ty: &Type) -> Option<Collection> {
        Collection::of(ty, &self.rust_types)
    }

    pub(in crate::types) fn collection_of_referent(&self, ty: &Type) -> Option<Collection> {
        Collection::of_referent(ty, &self.rust_types)
    }

    pub(in crate::types) fn collection(&self, ty: &Type) -> Collection {
        self.collection_of_referent(ty)
            .unwrap_or_else(|| panic!("the fixture types name collections"))
    }

    pub(in crate::types) fn map() -> Type {
        Type::Named(Self::MAP, vec![Self::key(), Type::bool()])
    }

    pub(in crate::types) fn key() -> Type {
        Type::Primitive(PrimitiveType::Str)
    }

    pub(in crate::types) fn vector_of(element: Type) -> Type {
        Type::Named(Self::VECTOR, vec![element])
    }

    pub(in crate::types) fn nested_vector_of(depth: usize, element: &Type) -> Type {
        (0..depth).fold(element.clone(), |ty, _| Self::vector_of(ty))
    }

    pub(in crate::types) fn slice_of(element: Type) -> Type {
        Type::Named(Self::SLICE, vec![element])
    }

    pub(in crate::types) fn element() -> Type {
        Type::Primitive(PrimitiveType::U8)
    }
}
