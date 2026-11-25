use super::{CollectionAssumptions, CollectionShapes, SyntheticCalls, Unknowns};

pub(in crate::types) trait VectorPushTyping:
    Unknowns + CollectionShapes + CollectionAssumptions + SyntheticCalls
{
}

impl<C> VectorPushTyping for C where
    C: Unknowns + CollectionShapes + CollectionAssumptions + SyntheticCalls
{
}
