use super::{MapBorrows, MapKeys, SyntheticCalls, Unknowns};

pub(in crate::types) trait MapWriteTyping:
    MapKeys + Unknowns + SyntheticCalls + MapBorrows
{
}

impl<C> MapWriteTyping for C where C: MapKeys + Unknowns + SyntheticCalls + MapBorrows {}
