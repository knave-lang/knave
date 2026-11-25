use super::{MapKeys, SyntheticCalls, Unknowns};

pub(in crate::types) trait MapWriteTyping:
    MapKeys + Unknowns + SyntheticCalls
{
}

impl<C> MapWriteTyping for C where C: MapKeys + Unknowns + SyntheticCalls {}
