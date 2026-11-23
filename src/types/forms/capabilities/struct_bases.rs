use crate::diagnostic::Diagnostic;
use crate::types::ty::Type;

pub(in crate::types) trait StructBases {
    fn explain_struct_base(&self, error: Diagnostic, base: &Type) -> Diagnostic;
}
