use crate::source::Span;
use crate::types::collection::Collection;
use crate::types::ty::Type;

pub(in crate::types) trait MapBorrows {
    fn check_borrow(&mut self, map: &Collection, query: &Type, span: Span);
}
