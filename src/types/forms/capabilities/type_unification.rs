use crate::source::Span;
use crate::types::ty::Type;
use crate::types::unifier::UnifyError;

pub(in crate::types) trait TypeUnification {
    fn unify(&mut self, left: &Type, right: &Type, span: Span) -> Result<(), UnifyError>;
}
