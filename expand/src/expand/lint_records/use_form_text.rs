use crate::reader::{ListClose, Syntax};
use crate::source::{SourceMap, Span};

pub(in crate::expand) struct UseFormText {
    head: Span,
    close: usize,
}

impl UseFormText {
    pub(in crate::expand) fn new(sources: &SourceMap, form: Span, head: Span) -> Self {
        let close = if Self::opens_with(form, head) {
            form.range().end - Syntax::CLOSE.len_utf8()
        } else {
            ListClose::after(sources.file(head.source()), head.range().end)
        };
        Self { head, close }
    }

    fn opens_with(form: Span, head: Span) -> bool {
        form.source() == head.source()
            && form.range().start + Syntax::OPEN.len_utf8() == head.range().start
    }

    pub(in crate::expand) fn holds(&self, name: Span) -> bool {
        name.source() == self.head.source()
            && name.range().start >= self.head.range().end
            && name.range().start < self.close
    }
}

#[cfg(test)]
mod tests {
    use super::UseFormText;
    use crate::source::{SourceId, SourceMap, Span};

    struct UseFormFixture {
        sources: SourceMap,
        source: SourceId,
    }

    impl UseFormFixture {
        const TEXT: &'static str = "(u (use format shout) pad-left) ; (use x\n(use y ; a )\n z)";

        fn new() -> Self {
            let mut sources = SourceMap::default();
            let source = sources.add("use", Self::TEXT);
            Self { sources, source }
        }

        fn holds(&self, head: &str, name: &str) -> bool {
            let source = self.source;
            let span = |word: &str, after: usize| {
                let start = after
                    + Self::TEXT[after..]
                        .find(word)
                        .expect("the text holds the word");
                Span::new(source, start..start + word.len())
            };
            let head = span(head, 0);
            let name = span(name, head.range().end);
            let rebuilt = Span::new(source, 0..Self::TEXT.len());
            UseFormText::new(&self.sources, rebuilt, head).holds(name)
        }

        fn holds_in_written_form(&self, name: &str) -> bool {
            let form = Span::new(self.source, 3..21);
            let head = Span::new(self.source, 4..7);
            let start = Self::TEXT.find(name).expect("the text holds the name");
            let name = Span::new(self.source, start..start + name.len());
            UseFormText::new(&self.sources, form, head).holds(name)
        }
    }

    #[test]
    fn a_written_use_form_closes_where_its_span_ends() {
        let fixture = UseFormFixture::new();
        assert!(fixture.holds_in_written_form("shout"));
        assert!(!fixture.holds_in_written_form("pad-left"));
    }

    #[test]
    fn a_name_inside_the_use_form_is_held() {
        assert!(UseFormFixture::new().holds("use", "shout"));
    }

    #[test]
    fn a_name_after_the_use_form_closes_is_not_held() {
        assert!(!UseFormFixture::new().holds("use", "pad-left"));
    }

    #[test]
    fn a_parenthesis_in_a_comment_does_not_close_the_form() {
        assert!(UseFormFixture::new().holds("use y", "z"));
    }
}
