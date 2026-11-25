use super::{CollectionAssumptions, PreludeShapes, SyntheticCalls, Unknowns};

pub(in crate::types) trait SliceGetTyping:
    Unknowns + PreludeShapes + CollectionAssumptions + SyntheticCalls
{
}

impl<C> SliceGetTyping for C where
    C: Unknowns + PreludeShapes + CollectionAssumptions + SyntheticCalls
{
}
