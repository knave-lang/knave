use super::ty::Type;

#[derive(Clone)]
pub(super) struct RecurTarget {
    pub(super) parameters: Vec<Type>,
    pub(super) result: Type,
}
