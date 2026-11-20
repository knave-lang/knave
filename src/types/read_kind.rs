#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadKind {
    Value,
    StoredBorrow,
    BorrowOfStruct,
}

impl ReadKind {
    pub fn getter_prefix(self) -> &'static str {
        match self {
            ReadKind::BorrowOfStruct => "&",
            ReadKind::Value | ReadKind::StoredBorrow => "",
        }
    }
}
