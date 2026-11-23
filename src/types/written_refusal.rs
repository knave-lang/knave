use super::read_kind::ReadKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WrittenRefusal {
    Read(ReadRefusal),
    Use(UseRefusal),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReadRefusal {
    Differing,
    Function,
    Open,
    OpenStruct,
    Otherwise,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UseRefusal {
    ChangesField,
    MovesOwner,
    BorrowLeaves,
    ValueLeaves,
    NameTaken,
}

impl UseRefusal {
    pub(super) fn leaving(kind: ReadKind) -> Self {
        match kind {
            ReadKind::Value => Self::ValueLeaves,
            ReadKind::StoredBorrow | ReadKind::BorrowOfStruct => Self::BorrowLeaves,
        }
    }
}
