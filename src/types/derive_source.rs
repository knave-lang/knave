use crate::source::Span;

#[derive(Clone, Copy)]
pub(super) enum DeriveSource {
    Inferred,
    Listed(Span),
}
