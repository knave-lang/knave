use crate::types::collection::Collection;
use crate::types::ty::Type;

pub(in crate::types) trait CollectionShapes {
    fn collection_of(&self, ty: &Type) -> Option<Collection>;
    fn nested_vector_of(&self, depth: usize, element: &Type) -> Type;
}
