#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictKind {
    Change,
    Move,
    BorrowDuringMut,
}
