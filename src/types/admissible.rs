use super::ty::Type;

#[derive(Debug, PartialEq)]
pub(super) enum Admissible {
    Unbounded,
    Open(Vec<Type>),
    Closed(Vec<Type>),
}
