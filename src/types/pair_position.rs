use super::assumed_collections::CollectionAssumption;
use crate::core::MapOperation;

#[derive(Clone, Copy)]
pub enum PairPosition {
    First,
    Second,
}

impl PairPosition {
    pub fn index(self) -> usize {
        match self {
            PairPosition::First => 0,
            PairPosition::Second => 1,
        }
    }

    pub fn field_name(self) -> &'static str {
        match self {
            PairPosition::First => "0",
            PairPosition::Second => "1",
        }
    }

    pub fn operation(self) -> MapOperation {
        match self {
            PairPosition::First => MapOperation::First,
            PairPosition::Second => MapOperation::Second,
        }
    }

    pub fn slice_assumption(self) -> CollectionAssumption {
        CollectionAssumption::PairPartCollection(self.operation())
    }
}
