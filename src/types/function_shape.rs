use super::ty::Type;

pub(super) struct FunctionShape {
    pub(super) parameters: Vec<Type>,
    pub(super) result: Type,
}
