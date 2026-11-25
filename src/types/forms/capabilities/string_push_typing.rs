use super::{ArgumentPassing, SyntheticCalls, TypeDisplays, Unknowns};

pub(in crate::types) trait StringPushTyping:
    Unknowns + ArgumentPassing + SyntheticCalls + TypeDisplays
{
}

impl<C> StringPushTyping for C where C: Unknowns + ArgumentPassing + SyntheticCalls + TypeDisplays {}
