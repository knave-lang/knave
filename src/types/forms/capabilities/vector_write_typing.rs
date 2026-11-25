use super::{PlaceWrites, PreludeShapes, TypeFitting, Unknowns};

pub(in crate::types) trait VectorWriteTyping:
    Unknowns + PreludeShapes + TypeFitting + PlaceWrites
{
}

impl<C> VectorWriteTyping for C where C: Unknowns + PreludeShapes + TypeFitting + PlaceWrites {}
