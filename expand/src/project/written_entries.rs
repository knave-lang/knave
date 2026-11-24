use crate::source::Span;

pub(crate) trait WrittenEntries {
    fn quoted_call(&self, site: Span) -> String;
    fn location(&self, site: Span) -> String;
}
