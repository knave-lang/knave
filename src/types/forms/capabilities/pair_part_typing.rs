use super::{ArgumentTyping, CalleeShapes, ChildTyping, CollectionAssumptions, PairParts};

pub(in crate::types) trait PairPartTyping:
    PairParts + CollectionAssumptions + ChildTyping + CalleeShapes + ArgumentTyping
{
}

impl<C> PairPartTyping for C where
    C: PairParts + CollectionAssumptions + ChildTyping + CalleeShapes + ArgumentTyping
{
}
