use crate::types::collection::Collection;
use crate::types::ty::Type;

pub(in crate::types) trait CollectionLookups {
    fn collection_of(&self, ty: &Type) -> Option<Collection>;
    fn collection_of_referent(&self, ty: &Type) -> Option<Collection>;
    fn collection_item(&self, ty: &Type) -> Option<(Type, Type)>;
}
