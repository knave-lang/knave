use rustc_hash::FxHashSet;

use crate::core::BindingId;

#[derive(Debug, PartialEq, Eq)]
pub struct ResultRegion {
    borrowed: FxHashSet<BindingId>,
    higher_ranked: FxHashSet<BindingId>,
}

impl ResultRegion {
    pub fn new(borrowed: FxHashSet<BindingId>, higher_ranked: FxHashSet<BindingId>) -> Self {
        Self {
            borrowed,
            higher_ranked,
        }
    }
}
