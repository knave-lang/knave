use crate::source::{SourceId, Span};

pub struct EntrySpan;

impl EntrySpan {
    pub fn start() -> Span {
        Span::new(SourceId::new(0), 0..0)
    }
}
