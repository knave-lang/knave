use super::held_bound::HeldBound;
use crate::diagnostic::Diagnostics;

pub(super) struct OpenRoundEnd {
    pub(super) missing: Diagnostics,
    pub(super) held: Vec<HeldBound>,
}
