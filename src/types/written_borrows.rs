use super::typed::{TypedExpr, TypedExprKind};

pub struct WrittenBorrows;

impl WrittenBorrows {
    pub fn under(value: &TypedExpr, levels: usize) -> Option<&TypedExpr> {
        (0..levels).try_fold(Self::without_dereferenced(value), |value, _| {
            match &value.kind {
                TypedExprKind::Borrow(_, place) => Some(place.as_ref()),
                _ => None,
            }
        })
    }

    pub fn without_dereferenced(value: &TypedExpr) -> &TypedExpr {
        match &value.kind {
            TypedExprKind::Deref(inner) => match &inner.kind {
                TypedExprKind::Borrow(_, place) => Self::without_dereferenced(place),
                _ => value,
            },
            _ => value,
        }
    }
}
