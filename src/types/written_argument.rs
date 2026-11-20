use super::ty::Type;

pub(super) struct WrittenArgument {
    pub(super) position: usize,
    pub(super) resolved: Type,
    pub(super) depth: usize,
}
