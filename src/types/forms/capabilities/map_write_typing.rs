use super::{MapKeys, PlaceWrites, Unknowns};

pub(in crate::types) trait MapWriteTyping:
    MapKeys + Unknowns + PlaceWrites
{
}

impl<C> MapWriteTyping for C where C: MapKeys + Unknowns + PlaceWrites {}
