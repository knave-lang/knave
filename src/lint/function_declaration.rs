use super::declared_signature::DeclaredSignature;
use crate::core::BindingId;
use crate::source::Span;

#[derive(Clone)]
pub struct FunctionDeclaration {
    pub function: BindingId,
    pub span: Span,
    pub signature: DeclaredSignature,
    pub parameters: Vec<BindingId>,
}

impl FunctionDeclaration {
    pub fn needs_a_trial(&self) -> bool {
        !self.signature.writes_lifetime()
    }
}
