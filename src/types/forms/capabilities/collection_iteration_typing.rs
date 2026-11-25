use super::{CollectionLookups, LiteralUnknowns, MapKeys, SliceIterationTyping, TypeDisplays};

pub(in crate::types) trait CollectionIterationTyping:
    SliceIterationTyping + LiteralUnknowns + CollectionLookups + MapKeys + TypeDisplays
{
}

impl<C> CollectionIterationTyping for C where
    C: SliceIterationTyping + LiteralUnknowns + CollectionLookups + MapKeys + TypeDisplays
{
}
