use super::ty::Type;
use super::typed::TypedBinder;
use crate::core::{BindingId, ReferenceKind, TypeExpr, TypeExprKind};
use crate::source::Span;

impl TypedBinder {
    pub fn new(binding: BindingId, ty: Type, span: Span) -> Self {
        Self {
            binding,
            ty,
            span,
            annotated: false,
            written_view: false,
        }
    }

    pub fn writes_a_view(annotation: Option<&TypeExpr>) -> bool {
        annotation.is_some_and(|written| match written.kind() {
            TypeExprKind::Reference(ReferenceKind::Shared, ..) => true,
            TypeExprKind::Reference(ReferenceKind::Mutable, _, inner) => {
                Self::writes_a_view(Some(inner))
            }
            _ => false,
        })
    }

    pub fn with_type(&self, ty: Type) -> Self {
        Self { ty, ..self.clone() }
    }
}
