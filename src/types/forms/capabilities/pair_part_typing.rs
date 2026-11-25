use super::{
    ArgumentTyping, CalleeFunctions, ChildTyping, CollectionAssumptions, PairParts, Unknowns,
};

pub(in crate::types) trait PairPartTyping:
    Unknowns + PairParts + CollectionAssumptions + ChildTyping + CalleeFunctions + ArgumentTyping
{
}

impl<C> PairPartTyping for C where
    C: Unknowns
        + PairParts
        + CollectionAssumptions
        + ChildTyping
        + CalleeFunctions
        + ArgumentTyping
{
}
