use super::{PlaceWrites, Unknowns};

pub(in crate::types) trait WholePlaceWriteTyping: Unknowns + PlaceWrites {}

impl<C> WholePlaceWriteTyping for C where C: Unknowns + PlaceWrites {}
