use super::ty::Bound;

pub(super) enum BoundProof {
    Proven,
    Unproven(Bound),
}
