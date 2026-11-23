use super::inferred_read::InferredRead;
use crate::core::BindingId;
use crate::source::Span;

#[derive(Clone, Copy, Debug)]
pub enum BoundOrigin {
    BorrowCopied(Span),
    ImplBody(Span),
    Included(BindingId),
    InferredRead(InferredRead),
}

impl BoundOrigin {
    pub fn including_trait(&self) -> Option<BindingId> {
        match self {
            Self::Included(including) => Some(*including),
            Self::BorrowCopied(_) | Self::ImplBody(_) | Self::InferredRead(_) => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BoundOrigins(Vec<BoundOrigin>);

impl BoundOrigins {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn borrow_copied(copied: Span) -> Self {
        Self(vec![BoundOrigin::BorrowCopied(copied)])
    }

    pub fn add(&mut self, origin: BoundOrigin) {
        self.0.push(origin);
    }

    pub fn inherit(&mut self, required: &BoundOrigins) {
        self.0.extend(required.0.iter().copied());
    }

    pub fn is_needed_by_impl_body(&self) -> bool {
        self.0
            .iter()
            .any(|origin| matches!(origin, BoundOrigin::ImplBody(_)))
    }

    pub fn generalize(&mut self) {
        for origin in &mut self.0 {
            if let BoundOrigin::InferredRead(read) = *origin {
                *origin = BoundOrigin::InferredRead(read.generalized());
            }
        }
    }

    pub fn as_slice(&self) -> &[BoundOrigin] {
        &self.0
    }
}
