use std::collections::BTreeSet;
use std::fmt;

use crate::verdict::CitedPlace;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FindingSignature {
    oracle: &'static str,
    cited: BTreeSet<CitedPlace>,
}

impl FindingSignature {
    pub fn new(oracle: &'static str, cited: BTreeSet<CitedPlace>) -> Self {
        Self { oracle, cited }
    }
}

impl fmt::Display for FindingSignature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.oracle)?;
        self.cited
            .iter()
            .try_for_each(|place| write!(formatter, " {place}"))
    }
}
