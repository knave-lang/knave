use crate::core::BindingId;
use crate::source::Span;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(super) struct StructuralPathClaim {
    owner: BindingId,
    clause: Span,
}

impl StructuralPathClaim {
    pub(super) fn new(owner: BindingId, clause: Span) -> Self {
        Self { owner, clause }
    }

    pub(super) fn owner(self) -> BindingId {
        self.owner
    }

    pub(super) fn clause(self) -> Span {
        self.clause
    }
}
