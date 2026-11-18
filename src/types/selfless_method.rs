use crate::core::{BindingId, TraitMethod, TypeExpr, TypeExprKind};
use crate::source::Span;

#[derive(Clone, Copy)]
pub(super) struct SelflessMethod {
    trait_binding: BindingId,
    signature: Span,
}

impl SelflessMethod {
    pub(super) fn of(
        method: &TraitMethod,
        trait_binding: BindingId,
        self_variable: BindingId,
    ) -> Option<Self> {
        if Self::mentions(&method.signature, self_variable) {
            return None;
        }
        Some(Self {
            trait_binding,
            signature: method.signature.span(),
        })
    }

    pub(super) fn trait_binding(&self) -> BindingId {
        self.trait_binding
    }

    pub(super) fn signature(&self) -> Span {
        self.signature
    }

    fn mentions(signature: &TypeExpr, variable: BindingId) -> bool {
        match signature.kind() {
            TypeExprKind::Named(binding, _) if *binding == variable => true,
            TypeExprKind::Where(_, inner) => Self::mentions(inner, variable),
            TypeExprKind::With(outer, inner) => {
                Self::mentions(outer, variable) || Self::mentions(inner, variable)
            }
            _ => signature
                .parts()
                .into_iter()
                .any(|part| Self::mentions(part, variable)),
        }
    }
}
