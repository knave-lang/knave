use super::{
    ChildTyping, CollectionLookups, FieldMoves, MapKeys, PreludeShapes, StructBases, TypeFitting,
    Unknowns,
};

pub(in crate::types) trait AssociationTyping:
    ChildTyping
    + TypeFitting
    + Unknowns
    + CollectionLookups
    + PreludeShapes
    + MapKeys
    + StructBases
    + FieldMoves
{
}

impl<C> AssociationTyping for C where
    C: ChildTyping
        + TypeFitting
        + Unknowns
        + CollectionLookups
        + PreludeShapes
        + MapKeys
        + StructBases
        + FieldMoves
{
}
