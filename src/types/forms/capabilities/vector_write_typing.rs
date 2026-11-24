use super::{CollectionShapes, SliceShapes, SyntheticCalls, TypeFitting, Unknowns};

pub(in crate::types) trait VectorWriteTyping:
    Unknowns + CollectionShapes + SliceShapes + TypeFitting + SyntheticCalls
{
}

impl<C> VectorWriteTyping for C where
    C: Unknowns + CollectionShapes + SliceShapes + TypeFitting + SyntheticCalls
{
}
