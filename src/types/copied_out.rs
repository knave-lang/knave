use super::typed::{Borrow, TypedArgument, TypedExpr};
use super::written_borrows::WrittenBorrows;

pub enum CopiedOut<'a> {
    IndexedPlace { dereferences: usize },
    DereferencedPlace(&'a TypedExpr),
    CollapsedPlace(&'a TypedExpr),
}

impl<'a> CopiedOut<'a> {
    pub fn of(
        argument: &'a TypedArgument,
        indexes_in_place: impl FnOnce(&TypedExpr) -> bool,
    ) -> Option<Self> {
        Self::indexed(argument, indexes_in_place).or_else(|| Self::written(argument))
    }

    fn indexed(
        argument: &TypedArgument,
        indexes_in_place: impl FnOnce(&TypedExpr) -> bool,
    ) -> Option<Self> {
        let levels = match argument.borrow {
            Borrow::Deref(levels) => levels,
            Borrow::Collapse(1) => 1,
            _ => return None,
        };
        indexes_in_place(&argument.value).then(|| Self::IndexedPlace {
            dereferences: levels - 1,
        })
    }

    fn written(argument: &'a TypedArgument) -> Option<Self> {
        let value = &argument.value;
        match argument.borrow {
            Borrow::Deref(levels) => {
                WrittenBorrows::under(value, levels).map(Self::DereferencedPlace)
            }
            Borrow::Collapse(levels) => {
                WrittenBorrows::under(value, levels).map(Self::CollapsedPlace)
            }
            _ => None,
        }
    }
}
