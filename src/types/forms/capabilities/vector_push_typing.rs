use super::{CollectionAssumptions, PreludeShapes, SyntheticCalls, Unknowns};

pub(in crate::types) trait VectorPushTyping:
    Unknowns + PreludeShapes + CollectionAssumptions + SyntheticCalls
{
}

impl<C> VectorPushTyping for C where
    C: Unknowns + PreludeShapes + CollectionAssumptions + SyntheticCalls
{
}
