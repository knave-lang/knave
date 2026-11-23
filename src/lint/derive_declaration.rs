use crate::core::{BindingId, ModulePath};
use crate::source::Span;

pub struct DeriveDeclaration {
    pub target: BindingId,
    pub module: ModulePath,
    pub span: Span,
    pub listed: Vec<BindingId>,
}
