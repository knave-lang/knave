use crate::types::collection::Collection;
use crate::types::ty::Type;

pub(in crate::types) trait ReferentCollections {
    fn collection_of_referent(&self, ty: &Type) -> Option<Collection>;
}
