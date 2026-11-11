#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiverKind {
    Owned,
    Shared,
    Mutable,
}
