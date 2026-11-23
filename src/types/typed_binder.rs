use super::ty::Type;
use super::typed::TypedBinder;
use crate::core::{BindingId, PrimitiveType, ReferenceKind, TypeExpr, TypeExprKind};
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

    pub fn written_view(annotation: Option<&TypeExpr>, signature: Option<&Type>) -> bool {
        annotation.map_or_else(
            || signature.is_some_and(Self::signature_writes_a_view),
            Self::annotation_writes_a_view,
        )
    }

    pub fn annotation_writes_a_view(written: &TypeExpr) -> bool {
        match written.kind() {
            TypeExprKind::Reference(ReferenceKind::Shared, ..) => true,
            TypeExprKind::Reference(ReferenceKind::Mutable, _, inner) => {
                Self::annotation_writes_a_view(inner)
            }
            _ => false,
        }
    }

    pub fn signature_writes_a_view(written: &Type) -> bool {
        match written {
            Type::Ref(_, inner) => matches!(
                inner.as_ref(),
                Type::Primitive(PrimitiveType::Str | PrimitiveType::String)
            ),
            Type::Mut(_, inner) => Self::signature_writes_a_view(inner),
            _ => false,
        }
    }

    pub fn with_type(&self, ty: Type) -> Self {
        Self { ty, ..self.clone() }
    }
}
