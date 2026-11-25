use super::{PreludeShapes, SyntheticCalls, Unknowns};

pub(in crate::types) trait SliceIterationTyping:
    Unknowns + PreludeShapes + SyntheticCalls
{
}

impl<C> SliceIterationTyping for C where C: Unknowns + PreludeShapes + SyntheticCalls {}
