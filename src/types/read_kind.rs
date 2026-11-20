#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadKind {
    Value,
    StoredBorrow,
    BorrowOfStruct,
}
