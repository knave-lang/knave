use std::ops::Range;

use super::Reader;
use super::cursor::Cursor;
use super::form::{Form, FormKind};
use super::string_literal::StringLiteralReader;
use crate::source::{SourceFile, Span};

pub struct LiteralForms {
    decoded: SourceFile,
    placement: Placement,
}

enum Placement {
    AtWrittenRanges(Vec<Range<usize>>),
    AtLiteral(Range<usize>),
}

impl LiteralForms {
    pub fn new(enclosing: &SourceFile, literal: Range<usize>, value: &str) -> Self {
        Self {
            decoded: SourceFile::new(
                enclosing.id().pinned(),
                enclosing.name().to_string(),
                value.to_string(),
            ),
            placement: Placement::of(enclosing.text(), literal, value),
        }
    }

    pub fn read_at(&self, character: usize) -> Option<(Form, usize)> {
        let text = self.decoded.text();
        let offset = text.char_indices().nth(character)?.0;
        let form = Reader::read_form_at(&self.decoded, offset)?;
        let end = text[..form.span().range().end].chars().count();
        Some((self.placed(&form), end))
    }

    fn placed(&self, form: &Form) -> Form {
        let kind = match form.kind() {
            FormKind::List(items) => {
                FormKind::List(items.iter().map(|item| self.placed(item)).collect())
            }
            other => other.clone(),
        };
        Form::new(kind, self.placement.span_of(form.span()))
    }
}

impl Placement {
    fn of(file_text: &str, literal: Range<usize>, value: &str) -> Self {
        let mut cursor = Cursor::starting_at(&file_text[..literal.end], literal.start);
        let (read_value, written_ranges) = StringLiteralReader::read_written_ranges(&mut cursor);
        if read_value == value {
            Self::AtWrittenRanges(written_ranges)
        } else {
            Self::AtLiteral(literal)
        }
    }

    fn span_of(&self, decoded: Span) -> Span {
        let range = decoded.range();
        let placed = match self {
            Self::AtWrittenRanges(written_ranges) => {
                written_ranges[range.start].start..written_ranges[range.end - 1].end
            }
            Self::AtLiteral(literal) => literal.clone(),
        };
        Span::new(decoded.source(), placed)
    }
}

#[cfg(test)]
mod tests {
    use super::LiteralForms;
    use crate::source::{SourceId, SourceMap};

    struct LiteralFixture {
        sources: SourceMap,
        file: SourceId,
        text: &'static str,
    }

    impl LiteralFixture {
        fn new(text: &'static str) -> Self {
            let mut sources = SourceMap::default();
            let file = sources.add("doc.knv", text);
            Self {
                sources,
                file,
                text,
            }
        }

        fn offset_of(&self, part: &str) -> usize {
            self.text.find(part).expect("the text holds it")
        }

        fn forms(&self, value: &str) -> LiteralForms {
            let literal = self.offset_of("\"")..self.text.rfind('"').expect("a closing quote") + 1;
            LiteralForms::new(self.sources.file(self.file), literal, value)
        }

        fn written_at(&self, value: &str, character: usize) -> String {
            let (form, _) = self.forms(value).read_at(character).expect("a form reads");
            self.text[form.span().range()].to_string()
        }
    }

    #[test]
    fn a_form_after_an_escape_points_at_its_written_text() {
        let fixture = LiteralFixture::new(r#"(doc "a \"b\" (+ 1 2) end")"#);
        assert_eq!(fixture.written_at("a \"b\" (+ 1 2) end", 6), "(+ 1 2)");
    }

    #[test]
    fn a_form_that_ends_in_a_wide_character_covers_all_of_it() {
        let fixture = LiteralFixture::new(r#"(doc "café")"#);
        assert_eq!(fixture.written_at("café", 0), "café");
    }

    #[test]
    fn a_form_that_ends_in_an_escape_covers_all_of_it() {
        let fixture = LiteralFixture::new(r#"(doc "\"b\"")"#);
        assert_eq!(fixture.written_at("\"b\"", 0), r#"\"b\""#);
    }

    #[test]
    fn a_read_form_is_pinned_to_the_source_of_its_literal() {
        let fixture = LiteralFixture::new(r#"(doc "(x)")"#);
        let (form, _) = fixture.forms("(x)").read_at(0).expect("a form reads");
        assert!(form.span().is_pinned());
        assert_eq!(form.span().source().index(), fixture.file.index());
    }

    #[test]
    fn a_character_counts_once_whatever_its_width() {
        let fixture = LiteralFixture::new(r#"(doc "\u{e9}é (x)")"#);
        assert_eq!(fixture.written_at("éé (x)", 3), "(x)");
    }

    #[test]
    fn an_item_of_a_list_points_at_its_own_text() {
        let fixture = LiteralFixture::new(r#"(doc "(alpha beta)")"#);
        let (form, _) = fixture
            .forms("(alpha beta)")
            .read_at(0)
            .expect("a form reads");
        let items = form.as_list().expect("a list");
        assert_eq!(&fixture.text[items[1].span().range()], "beta");
    }

    #[test]
    fn reading_returns_the_character_after_the_form() {
        let fixture = LiteralFixture::new(r#"(doc "(x) y")"#);
        let (_, end) = fixture.forms("(x) y").read_at(0).expect("a form reads");
        assert_eq!(end, 3);
    }

    #[test]
    fn a_position_past_the_end_reads_nothing() {
        let fixture = LiteralFixture::new(r#"(doc "(x)")"#);
        assert!(fixture.forms("(x)").read_at(9).is_none());
    }

    #[test]
    fn an_unbalanced_form_reads_nothing() {
        let fixture = LiteralFixture::new(r#"(doc "(x")"#);
        assert!(fixture.forms("(x").read_at(0).is_none());
    }

    #[test]
    fn a_value_that_the_literal_does_not_spell_points_at_the_whole_literal() {
        let fixture = LiteralFixture::new(r#"(doc "other")"#);
        assert_eq!(fixture.written_at("(x)", 0), r#""other""#);
    }
}
