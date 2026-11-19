use super::assumed_collections::CollectionAssumption;
use crate::core::MapOperation;

pub struct AssumedCollectionProblems;

impl AssumedCollectionProblems {
    pub fn help(assumption: CollectionAssumption, function: &str, signature: &str) -> String {
        format!(
            "{}, so write a `type` signature for `{function}`, such as `(type {function} {signature})`",
            Self::reading(assumption),
        )
    }

    fn reading(assumption: CollectionAssumption) -> String {
        match assumption {
            CollectionAssumption::PushTarget => {
                "a push reads a target of unknown type as a vector".to_string()
            }
            CollectionAssumption::GetCollection => {
                "a lookup reads a collection of unknown type as a vector or a slice".to_string()
            }
            CollectionAssumption::PairPartCollection(position) => format!(
                "`{}` reads a value of unknown type as a vector or a slice, not a pair",
                MapOperation::from(position).keyword()
            ),
        }
    }
}
