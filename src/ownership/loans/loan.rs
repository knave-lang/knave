use super::lending::Lending;
use super::point_id::PointId;
use crate::core::BindingArena;
use crate::source::Span;

pub struct Loan {
    lending: Lending,
    made_at: PointId,
}

impl Loan {
    pub fn new(lending: Lending, made_at: PointId) -> Self {
        Self { lending, made_at }
    }

    pub fn made_at(&self) -> PointId {
        self.made_at
    }

    pub fn span(&self) -> Span {
        self.lending.span()
    }

    pub fn written(&self, bindings: &BindingArena) -> String {
        self.lending.written(bindings)
    }
}
