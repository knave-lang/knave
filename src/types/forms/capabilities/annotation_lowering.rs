use crate::core::TypeExpr;
use crate::diagnostic::Diagnostic;
use crate::types::ty::Type;

pub(in crate::types) trait AnnotationLowering {
    fn lower_annotation(&mut self, annotation: &TypeExpr) -> Result<Type, Diagnostic>;
}
