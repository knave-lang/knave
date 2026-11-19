use super::ty::Type;

pub(super) struct GroupDepths {
    pub(super) positions: Vec<usize>,
    pub(super) resolved: Vec<Type>,
    pub(super) depths: Vec<usize>,
}
