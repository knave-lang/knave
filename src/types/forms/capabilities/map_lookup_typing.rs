use super::{MapKeys, PreludeShapes, SyntheticCalls, Unknowns};

pub(in crate::types) trait MapLookupTyping:
    MapKeys + Unknowns + PreludeShapes + SyntheticCalls
{
}

impl<C> MapLookupTyping for C where C: MapKeys + Unknowns + PreludeShapes + SyntheticCalls {}
