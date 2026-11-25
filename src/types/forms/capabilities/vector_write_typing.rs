use super::{CollectionShapes, PlaceWrites, TypeFitting, Unknowns};

pub(in crate::types) trait VectorWriteTyping:
    Unknowns + CollectionShapes + TypeFitting + PlaceWrites
{
}

impl<C> VectorWriteTyping for C where C: Unknowns + CollectionShapes + TypeFitting + PlaceWrites {}
