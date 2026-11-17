use rustc_hash::FxHashSet;

use crate::core::BindingId;
use crate::diagnostic::Diagnostic;

pub enum PendingDiagnostic {
    Final(Diagnostic),
    BorrowOffered {
        offered: Diagnostic,
        owned: Diagnostic,
        bound: BindingId,
    },
}

impl From<Diagnostic> for PendingDiagnostic {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Final(diagnostic)
    }
}

impl PendingDiagnostic {
    pub fn settled(self, moved_names: &FxHashSet<BindingId>) -> Diagnostic {
        match self {
            Self::Final(diagnostic) => diagnostic,
            Self::BorrowOffered { owned, bound, .. } if moved_names.contains(&bound) => owned,
            Self::BorrowOffered { offered, .. } => offered,
        }
    }
}
