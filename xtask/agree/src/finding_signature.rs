use std::collections::BTreeSet;

use crate::verdict::Rejection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindingSignature {
    oracle: &'static str,
    cited: BTreeSet<String>,
}

impl FindingSignature {
    pub fn new(oracle: &'static str, evidence: &[Rejection]) -> Self {
        Self {
            oracle,
            cited: evidence.iter().map(Rejection::to_string).collect(),
        }
    }
}
