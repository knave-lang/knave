use super::ty::Type;
use super::typed::{TypedExpr, TypedExprKind, TypedStructExpr};
use crate::core::FieldLabel;
use crate::source::Span;

pub(super) struct FieldReadExpr {
    value: TypedExpr,
    label: FieldLabel,
}

impl FieldReadExpr {
    pub(super) fn new(value: TypedExpr, label: &FieldLabel) -> Self {
        Self {
            value,
            label: label.clone(),
        }
    }

    pub(super) fn typed(self, ty: Type, span: Span) -> TypedExpr {
        let kind = TypedExprKind::Struct(TypedStructExpr::Field(Box::new(self.value), self.label));
        TypedExpr { kind, ty, span }
    }
}
