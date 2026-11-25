use super::holdings::Holdings;
use super::loan_place::LoanPlace;
use super::point_id::PointId;
use super::recorded_store::RecordedStore;
use super::value_flow::ValueFlow;
use crate::core::BindingId;
use crate::types::{TypedArgument, TypedExpr, TypedExprKind};

pub struct StoreTarget {
    root: BindingId,
}

impl StoreTarget {
    pub fn of(argument: &TypedArgument) -> Option<Self> {
        let place = LoanPlace::of(Self::borrowed(&argument.value))?;
        Some(Self { root: place.root() })
    }

    fn borrowed(value: &TypedExpr) -> &TypedExpr {
        match &value.kind {
            TypedExprKind::Borrow(_, borrowed) => Self::borrowed(borrowed),
            _ => value,
        }
    }

    pub fn receive(&self, point: PointId, stored: Holdings, values: &mut ValueFlow<'_>) {
        values.store(RecordedStore {
            point,
            target: self.root,
            held: stored,
        });
    }
}
