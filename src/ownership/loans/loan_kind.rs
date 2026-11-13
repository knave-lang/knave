use super::access::{Access, Borrowing, Change};
use crate::core::ReferenceKind;

pub trait LoanKind {
    fn access(&self) -> &'static dyn Access;

    fn written_prefix(&self) -> &'static str;
}

pub struct SharedLoan;

pub struct MutableLoan;

impl dyn LoanKind {
    pub fn of(kind: ReferenceKind) -> &'static dyn LoanKind {
        match kind {
            ReferenceKind::Shared => &SharedLoan,
            ReferenceKind::Mutable => &MutableLoan,
        }
    }
}

impl LoanKind for SharedLoan {
    fn access(&self) -> &'static dyn Access {
        &Borrowing
    }

    fn written_prefix(&self) -> &'static str {
        ""
    }
}

impl LoanKind for MutableLoan {
    fn access(&self) -> &'static dyn Access {
        &Change
    }

    fn written_prefix(&self) -> &'static str {
        "mut "
    }
}
