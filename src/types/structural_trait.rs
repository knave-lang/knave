use super::ty::Type;
use crate::core::{BindingId, StructuralClause};

#[derive(Clone)]
pub struct StructuralTrait {
    pub binding: BindingId,
    pub method: BindingId,
    pub parameters: Vec<Type>,
    pub result: Type,
    pub clause: StructuralClause,
}
