use crate::types::assumed_collections::CollectionAssumption;
use crate::types::typed::TypedExpr;

pub(in crate::types) trait CollectionAssumptions {
    fn assume_collection(&mut self, target: &TypedExpr, assumption: CollectionAssumption);
}
