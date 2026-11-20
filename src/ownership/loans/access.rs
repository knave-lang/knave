use super::conflict_wording::ConflictWording;
use crate::diagnostic::Diagnostic;

pub trait Access {
    fn verb(&self) -> &'static str;
    fn excludes_shared_loans(&self) -> bool;
    fn conflict(&self, wording: &dyn ConflictWording) -> Diagnostic;
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

    fn conflict(&self, wording: &dyn ConflictWording) -> Diagnostic {
        wording.changed_while_lent()
    }
}

impl Access for Move {
    fn verb(&self) -> &'static str {
        "move"
    }

    fn excludes_shared_loans(&self) -> bool {
        true
    }

    fn conflict(&self, wording: &dyn ConflictWording) -> Diagnostic {
        wording.moved_while_lent()
    }
}

impl Access for Borrowing {
    fn verb(&self) -> &'static str {
        "borrow"
    }

    fn excludes_shared_loans(&self) -> bool {
        false
    }

    fn conflict(&self, wording: &dyn ConflictWording) -> Diagnostic {
        wording.borrowed_while_lent_as_mut()
    }
}
