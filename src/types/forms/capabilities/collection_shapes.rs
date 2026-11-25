use crate::types::ty::Type;

pub(in crate::types) trait CollectionShapes {
    fn nested_vector_of(&self, depth: usize, element: &Type) -> Type;
    fn slice_of(&self, element: Type) -> Type;
}
