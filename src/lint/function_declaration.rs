use super::declared_signature::DeclaredSignature;
use crate::core::{BindingId, ModulePath};
use crate::source::Span;

#[derive(Clone)]
pub struct FunctionDeclaration {
    pub function: BindingId,
    pub module: ModulePath,
    pub span: Span,
    pub signature: DeclaredSignature,
    pub parameters: Vec<BindingId>,
}

impl FunctionDeclaration {
    pub fn needs_a_trial(&self) -> bool {
        !self.signature.writes_lifetime()
    }
}
