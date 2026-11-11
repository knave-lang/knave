use crate::core::BindingId;
use crate::source::Span;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct StructuralPathClaim {
    owner: BindingId,
    clause: Span,
}

impl StructuralPathClaim {
    pub fn new(owner: BindingId, clause: Span) -> Self {
        Self { owner, clause }
    }

    pub fn owner(self) -> BindingId {
        self.owner
    }

    pub fn clause(self) -> Span {
        self.clause
    }
}
