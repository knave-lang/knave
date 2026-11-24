use super::{
    CollectionShapes, LiteralKinds, ReferentCollections, TypeDisplays, TypeUnification, Unknowns,
};

pub(in crate::types) trait WriteCollectionTyping:
    Unknowns + LiteralKinds + TypeUnification + CollectionShapes + ReferentCollections + TypeDisplays
{
}

impl<C> WriteCollectionTyping for C where
    C: Unknowns
        + LiteralKinds
        + TypeUnification
        + CollectionShapes
        + ReferentCollections
        + TypeDisplays
{
}
