use rustc_hash::FxHashSet;

use super::display::TypeDisplay;
use super::parameter_names::ParameterNames;
use super::ty::Type;

pub(super) enum MissingRead {
    Open(String),
    Known(String),
}

impl MissingRead {
    const FIRST_OPEN_READ: char = 'r';

    pub(super) fn of(read: &Type, display: &TypeDisplay, taken: &FxHashSet<&str>) -> Self {
        match read {
            Type::Var(_) => Self::Open(ParameterNames::first_free_from(
                Self::FIRST_OPEN_READ,
                taken,
            )),
            known => Self::Known(display.show(known)),
        }
    }
}
