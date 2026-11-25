use crate::types::ty::Type;

pub(in crate::types) trait OptionShapes {
    fn option_of(&self, value: Type) -> Type;
}
