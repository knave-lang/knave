use super::{
    ChildTyping, CollectionShapes, FieldMoves, MapKeys, StructBases, TypeFitting, Unknowns,
};

pub(in crate::types) trait AssociationTyping:
    ChildTyping + TypeFitting + Unknowns + CollectionShapes + MapKeys + StructBases + FieldMoves
{
}

impl<C> AssociationTyping for C where
    C: ChildTyping + TypeFitting + Unknowns + CollectionShapes + MapKeys + StructBases + FieldMoves
{
}
