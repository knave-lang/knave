use super::{MapKeys, OptionShapes, SyntheticCalls, Unknowns};

pub(in crate::types) trait MapLookupTyping:
    MapKeys + Unknowns + OptionShapes + SyntheticCalls
{
}

impl<C> MapLookupTyping for C where C: MapKeys + Unknowns + OptionShapes + SyntheticCalls {}
