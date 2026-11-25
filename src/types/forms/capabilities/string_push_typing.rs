use super::{SyntheticCalls, TypeDisplays, Unknowns};

pub(in crate::types) trait StringPushTyping:
    Unknowns + SyntheticCalls + TypeDisplays
{
}

impl<C> StringPushTyping for C where C: Unknowns + SyntheticCalls + TypeDisplays {}
