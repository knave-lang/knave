use crate::diagnostic::Diagnostic;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::typed::AssignmentPath;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait PlaceWrites {
    fn type_place_write(
        &mut self,
        call: SyntheticCall<'_>,
        path: AssignmentPath,
    ) -> Result<TypedExpr, Diagnostic>;
}
