use super::{CollectionShapes, SyntheticCalls, TypeFitting, Unknowns};

pub(in crate::types) trait VectorWriteTyping:
    Unknowns + CollectionShapes + TypeFitting + SyntheticCalls
{
}

impl<C> VectorWriteTyping for C where C: Unknowns + CollectionShapes + TypeFitting + SyntheticCalls {}
