#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DerivedTrait {
    Written(&'static str),
    Structural,
}
