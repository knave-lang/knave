use crate::source::Span;
use crate::types::collection::Collection;

pub(in crate::types) trait MapKeys {
    fn require_map_key(&mut self, map: &Collection, span: Span);
}
