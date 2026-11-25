use super::{PreludeShapes, SyntheticCalls};

pub(in crate::types) trait MapIterationTyping:
    PreludeShapes + SyntheticCalls
{
}

impl<C> MapIterationTyping for C where C: PreludeShapes + SyntheticCalls {}
