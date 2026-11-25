use super::{CollectionLookups, CollectionShapes, TypeDisplays, TypeUnification, Unknowns};

pub(in crate::types) trait WriteCollectionTyping:
    Unknowns + TypeUnification + CollectionLookups + CollectionShapes + TypeDisplays
{
}

impl<C> WriteCollectionTyping for C where
    C: Unknowns + TypeUnification + CollectionLookups + CollectionShapes + TypeDisplays
{
}
