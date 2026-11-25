use super::ty::Type;
use super::typed::{Borrow, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{BindingId, ReferenceKind};

pub struct OwnedPlace<'a> {
    place: &'a TypedExpr,
}

impl<'a> OwnedPlace<'a> {
    pub fn of(argument: &'a TypedArgument) -> Option<Self> {
        match &argument.value.kind {
            TypedExprKind::Borrow(ReferenceKind::Mutable, place)
                if place.is_place() && !place.ty.is_reference() =>
            {
                Some(Self { place })
            }
            _ if Self::borrows_owned(argument) => Some(Self {
                place: &argument.value,
            }),
            _ => None,
        }
    }

    pub fn local(&self) -> Option<BindingId> {
        match self.place.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(local)) => Some(local),
            _ => None,
        }
    }

    fn borrows_owned(argument: &TypedArgument) -> bool {
        argument.borrow == Borrow::Mutable
            && argument.value.place_root().is_some()
            && !matches!(argument.value.ty, Type::Mut(..))
    }
}
