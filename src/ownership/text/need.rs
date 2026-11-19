#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Need {
    Owned,
    OwnedOrMixed,
    View,
}
