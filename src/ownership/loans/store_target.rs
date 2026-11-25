use super::holdings::Holdings;
use super::loan_place::LoanPlace;
use super::value_flow::ValueFlow;
use crate::core::BindingId;
use crate::types::TypedArgument;

pub struct StoreTarget {
    root: BindingId,
    through_reference: bool,
}

impl StoreTarget {
    pub fn of(argument: &TypedArgument) -> Option<Self> {
        let place = LoanPlace::of(&argument.value)?;
        Some(Self {
            root: place.root(),
            through_reference: argument.value.ty.is_reference(),
        })
    }

    pub fn receive(&self, stored: Holdings, values: &mut ValueFlow<'_>) {
        let held = if self.through_reference {
            stored.within()
        } else {
            stored
        };
        values.store(self.root, &held);
    }
}
