use crate::core::Conversion;
use crate::source::Span;

pub struct PlaceholderSite {
    pub position: usize,
    pub conversion: Conversion,
    pub span: Span,
}
