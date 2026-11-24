use crate::types::ty::Type;

pub(in crate::types) trait SliceShapes {
    fn slice_of(&self, element: Type) -> Type;
}
