use super::{CollectionAssumptions, CollectionShapes, OptionShapes, SyntheticCalls, Unknowns};

pub(in crate::types) trait SliceGetTyping:
    Unknowns + CollectionShapes + CollectionAssumptions + OptionShapes + SyntheticCalls
{
}

impl<C> SliceGetTyping for C where
    C: Unknowns + CollectionShapes + CollectionAssumptions + OptionShapes + SyntheticCalls
{
}
