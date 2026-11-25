use crate::types::ty::Type;

pub(in crate::types) trait PreludeShapes {
    fn nested_vector_of(&self, depth: usize, element: &Type) -> Type;
    fn slice_of(&self, element: Type) -> Type;
    fn option_of(&self, value: Type) -> Type;
    fn iterator_of(&self, item: Type) -> Type;
}
