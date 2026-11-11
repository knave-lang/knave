use super::lifted_temporary::LiftedTemporary;
use super::temporary_scope::TemporaryScope;
use crate::types::TypedExpr;

pub trait LiftPlacement {
    fn inner_scope(&self) -> TemporaryScope;

    fn settles_inside(&self) -> bool;

    fn take(&self, root: &mut TypedExpr, name: String) -> LiftedTemporary;
}
