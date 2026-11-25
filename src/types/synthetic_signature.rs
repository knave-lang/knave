use super::ty::Type;

pub(super) struct SyntheticSignature<'a> {
    pub(super) template: &'a str,
    pub(super) parameters: Vec<Type>,
    pub(super) result: Type,
}
