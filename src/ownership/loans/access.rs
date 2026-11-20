use super::conflict_kind::ConflictKind;

pub trait Access {
    fn verb(&self) -> &'static str;
    fn excludes_shared_loans(&self) -> bool;
    fn conflict_kind(&self) -> ConflictKind;
}

pub struct Change;

pub struct Move;

pub struct Borrowing;

impl Access for Change {
    fn verb(&self) -> &'static str {
        "change"
    }

    fn excludes_shared_loans(&self) -> bool {
        true
    }

    fn conflict_kind(&self) -> ConflictKind {
        ConflictKind::Change
    }
}

impl Access for Move {
    fn verb(&self) -> &'static str {
        "move"
    }

    fn excludes_shared_loans(&self) -> bool {
        true
    }

    fn conflict_kind(&self) -> ConflictKind {
        ConflictKind::Move
    }
}

impl Access for Borrowing {
    fn verb(&self) -> &'static str {
        "borrow"
    }

    fn excludes_shared_loans(&self) -> bool {
        false
    }

    fn conflict_kind(&self) -> ConflictKind {
        ConflictKind::BorrowDuringMut
    }
}
