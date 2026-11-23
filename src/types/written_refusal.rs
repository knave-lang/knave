#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WrittenRefusal {
    Read(ReadRefusal),
    Use(UseRefusal),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReadRefusal {
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
    NameTaken,
}
