use super::typed::{Borrow, TypedArgument, TypedExpr, TypedExprKind};
use crate::core::{BindingId, ReferenceKind};

pub struct ChangedRoot;

impl ChangedRoot {
    pub fn of_argument(argument: &TypedArgument) -> Option<BindingId> {
        let passed_mutably = matches!(argument.borrow, Borrow::Mutable | Borrow::Reborrow);
        Self::of_borrow(&argument.value).or_else(|| {
            passed_mutably
                .then(|| argument.value.place_root())
                .flatten()
        })
    }

    pub fn of_borrow(expr: &TypedExpr) -> Option<BindingId> {
        match &expr.kind {
            TypedExprKind::Borrow(ReferenceKind::Mutable, place) => place.place_root(),
            _ => None,
        }
    }
}
