use crate::source::Span;
use crate::types::collection::Collection;
use crate::types::ty::Type;

pub(in crate::types) trait MapKeys {
    fn require_map_key(&mut self, map: &Collection, span: Span);
    fn require_key_borrow(&mut self, map: &Collection, query: &Type, span: Span);
}
