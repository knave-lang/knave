use super::{CollectionLookups, PreludeShapes, TypeDisplays, TypeUnification, Unknowns};

pub(in crate::types) trait WriteCollectionTyping:
    Unknowns + TypeUnification + CollectionLookups + PreludeShapes + TypeDisplays
{
}

impl<C> WriteCollectionTyping for C where
    C: Unknowns + TypeUnification + CollectionLookups + PreludeShapes + TypeDisplays
{
}
