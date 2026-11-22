#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WrittenRefusal {
    FunctionRead,
    OpenRead,
    OpenStruct,
    ReadsOtherwise,
    ChangesField,
    MovesOwner,
    BorrowLeaves,
    NameTaken,
}
