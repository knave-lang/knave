use std::collections::BTreeSet;

use crate::verdict::CitedPlace;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindingSignature {
    oracle: &'static str,
    cited: BTreeSet<CitedPlace>,
}

impl FindingSignature {
    pub fn new(oracle: &'static str, cited: BTreeSet<CitedPlace>) -> Self {
        Self { oracle, cited }
    }
}
