use crate::source::Span;

#[derive(Clone, Copy, Default)]
pub(super) struct InferredSpan {
    innermost: Option<Span>,
}

impl InferredSpan {
    pub(super) fn around<C: AsMut<Self>, T>(
        context: &mut C,
        span: Span,
        infer: impl FnOnce(&mut C) -> T,
    ) -> T {
        let inner = Self {
            innermost: Some(span),
        };
        let enclosing = std::mem::replace(context.as_mut(), inner);
        let inferred = infer(context);
        *context.as_mut() = enclosing;
        inferred
    }

    pub(super) fn innermost(self) -> Option<Span> {
        self.innermost
    }
}
