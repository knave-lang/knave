use crate::source::Span;

#[derive(Clone, Copy, Debug)]
pub enum BoundOrigin {
    BorrowCopied(Span),
    ImplBody(Span),
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

    pub fn as_slice(&self) -> &[BoundOrigin] {
        &self.0
    }
}
