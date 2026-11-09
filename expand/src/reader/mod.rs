mod cursor;
mod form;
mod number;
mod problems;
mod string_literal;
mod syntax;
mod token;

pub use form::{Form, FormKind, Number, NumberValue, Symbol};
pub use token::{Token, TokenKind, TokenScan};

use std::ops::Range;

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::{SourceFile, Span};
use cursor::Cursor;
use number::{NumberError, NumberLiteral};
use problems::ReadProblems;
use string_literal::{StringLiteralReader, StringProblem};
use syntax::Syntax;

pub struct Reader<'a> {
    source: &'a SourceFile,
    cursor: Cursor<'a>,
    diagnostics: Diagnostics,
    string_ran_to_end: bool,
    open_lists: usize,
}

impl<'a> Reader<'a> {
    pub fn new(source: &'a SourceFile) -> Self {
        Self {
            source,
            cursor: Self::excerpt_cursor(source),
            diagnostics: Diagnostics::default(),
            string_ran_to_end: false,
            open_lists: 0,
        }
    }

    pub fn read_form_at(source: &'a SourceFile, offset: usize) -> Option<Form> {
        let excerpt = source.excerpt_range();
        if !excerpt.contains(&offset) {
            return None;
        }
        let mut reader = Self {
            cursor: Cursor::starting_at(&source.text()[..excerpt.end], offset),
            ..Self::new(source)
        };
        reader.read_one_form()
    }

    fn read_one_form(&mut self) -> Option<Form> {
        if !self.cursor.peek().is_some_and(Syntax::starts_form) {
            return None;
        }
        let form = self.read_form()?;
        self.diagnostics.is_empty().then_some(form)
    }

    fn excerpt_cursor(source: &'a SourceFile) -> Cursor<'a> {
        let excerpt = source.excerpt_range();
        Cursor::starting_at(&source.text()[..excerpt.end], excerpt.start)
    }

    pub fn read(mut self) -> Result<Vec<Form>, Diagnostics> {
        let forms = self.read_top_level();
        self.diagnostics.into_result(forms)
    }

    fn read_top_level(&mut self) -> Vec<Form> {
        let mut forms = Vec::new();
        while let Some(next) = self.skip_trivia() {
            if next == Syntax::CLOSE {
                self.skip_unmatched_close();
            } else {
                forms.extend(self.read_form());
            }
        }
        forms
    }

    fn skip_trivia(&mut self) -> Option<char> {
        loop {
            self.cursor.skip_whitespace();
            if self.cursor.peek()? != Syntax::COMMENT {
                return self.cursor.peek();
            }
            self.cursor.advance_to_line_end();
        }
    }

    fn read_form(&mut self) -> Option<Form> {
        let next = self.cursor.peek()?;
        if next == Syntax::OPEN {
            self.read_list()
        } else if next == Syntax::QUOTE {
            Some(self.read_string())
        } else if Syntax::is_reserved(next) {
            self.skip_reserved(next);
            None
        } else {
            self.read_atom()
        }
    }

    fn read_list(&mut self) -> Option<Form> {
        if self.open_lists == Form::NESTING_LIMIT {
            self.skip_list_past_limit();
            return None;
        }
        let start = self.cursor.position();
        self.cursor.advance();
        self.open_lists += 1;
        let items = self.read_list_items(start);
        self.open_lists -= 1;
        Some(Form::new(FormKind::List(items), self.span_from(start)))
    }

    fn skip_list_past_limit(&mut self) {
        let start = self.cursor.position();
        let open = self.span_of_char(start, Syntax::OPEN);
        self.report(ReadProblems::list_too_deep(open, Form::NESTING_LIMIT));
        self.skip_list(start);
    }

    fn skip_list(&mut self, start: usize) {
        let mut unclosed = 0;
        while let Some(next) = self.skip_trivia() {
            unclosed = self.skip_list_part(next, unclosed);
            if unclosed == 0 {
                return;
            }
        }
        self.report_unclosed_list(start);
    }

    fn skip_list_part(&mut self, next: char, unclosed: usize) -> usize {
        match next {
            Syntax::OPEN => {
                self.cursor.advance();
                unclosed + 1
            }
            Syntax::CLOSE => {
                self.cursor.advance();
                unclosed - 1
            }
            _ => {
                self.read_form();
                unclosed
            }
        }
    }

    fn read_list_items(&mut self, start: usize) -> Vec<Form> {
        let mut items = Vec::new();
        loop {
            match self.skip_trivia() {
                None => return self.unclosed_list(start, items),
                Some(Syntax::CLOSE) => return self.closed_list(items),
                Some(_) => items.extend(self.read_form()),
            }
        }
    }

    fn closed_list(&mut self, items: Vec<Form>) -> Vec<Form> {
        self.cursor.advance();
        items
    }

    fn unclosed_list(&mut self, start: usize, items: Vec<Form>) -> Vec<Form> {
        self.report_unclosed_list(start);
        items
    }

    fn report_unclosed_list(&mut self, start: usize) {
        if !self.string_ran_to_end {
            let open = self.span_of_char(start, Syntax::OPEN);
            self.report(ReadProblems::unclosed_list(open));
        }
    }

    fn read_string(&mut self) -> Form {
        let start = self.cursor.position();
        let literal = StringLiteralReader::read(&mut self.cursor);
        for problem in literal.problems {
            self.report_string_problem(problem, start);
        }
        Form::new(
            FormKind::String(literal.value.into()),
            self.span_from(start),
        )
    }

    fn report_string_problem(&mut self, problem: StringProblem, start: usize) {
        let diagnostic = match problem {
            StringProblem::Unterminated => {
                self.string_ran_to_end = true;
                ReadProblems::unterminated_string(self.span_of_char(start, Syntax::QUOTE))
            }
            StringProblem::UnknownEscape(range) => ReadProblems::unknown_escape(self.span(range)),
        };
        self.report(diagnostic);
    }

    fn read_atom(&mut self) -> Option<Form> {
        let start = self.cursor.position();
        let text = self.cursor.advance_atom();
        let span = self.span_from(start);
        let literal = NumberLiteral::new(text);
        if !literal.is_number_shaped() {
            return Some(Form::new(FormKind::Symbol(Symbol::new(text)), span));
        }
        match literal.parse() {
            Ok(number) => Some(Form::new(FormKind::Number(number), span)),
            Err(error) => self.skip_invalid_number(error, text, span),
        }
    }

    fn skip_invalid_number(&mut self, error: NumberError, text: &str, span: Span) -> Option<Form> {
        let diagnostic = match error {
            NumberError::Malformed => ReadProblems::malformed_number(text, span),
            NumberError::TooLarge => ReadProblems::number_too_large(text, span),
        };
        self.report(diagnostic);
        None
    }

    fn skip_unmatched_close(&mut self) {
        let close = self.span_of_char(self.cursor.position(), Syntax::CLOSE);
        self.cursor.advance();
        self.report(ReadProblems::unmatched_close(close));
    }

    fn skip_reserved(&mut self, character: char) {
        let span = self.span_of_char(self.cursor.position(), character);
        self.cursor.advance();
        self.report(ReadProblems::reserved_character(character, span));
    }

    fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn span_from(&self, start: usize) -> Span {
        self.span(start..self.cursor.position())
    }

    fn span_of_char(&self, start: usize, character: char) -> Span {
        self.span(start..start + character.len_utf8())
    }

    fn span(&self, range: Range<usize>) -> Span {
        Span::new(self.source.id(), range)
    }
}

#[cfg(test)]
mod tests {
    use super::{Form, Reader};
    use crate::source::{SourceId, SourceMap, TextPosition};

    struct ReadFixture {
        sources: SourceMap,
        file: SourceId,
    }

    impl ReadFixture {
        const TEXT: &str = "(defn area (w h) (* w h))\n; a note\n(defn half (x) (/ x 2))\n";

        fn new() -> Self {
            let mut sources = SourceMap::default();
            let file = sources.add("geometry.knv", Self::TEXT);
            Self { sources, file }
        }

        fn offset_of(text: &str) -> usize {
            Self::TEXT.find(text).expect("the text holds it")
        }

        fn form_at(&self, offset: usize) -> Option<Form> {
            Reader::read_form_at(self.sources.file(self.file), offset)
        }

        fn whole_file(&self) -> Vec<Form> {
            Reader::new(self.sources.file(self.file))
                .read()
                .unwrap_or_else(|_| panic!("the file reads"))
        }
    }

    #[test]
    fn a_form_in_the_middle_of_a_file_reads_alone() {
        let fixture = ReadFixture::new();
        let form = fixture.form_at(ReadFixture::offset_of("(defn half"));
        assert_eq!(
            form.map(|form| form.to_string()).as_deref(),
            Some("(defn half (x) (/ x 2))")
        );
    }

    #[test]
    fn a_nested_form_reads_alone() {
        let fixture = ReadFixture::new();
        let form = fixture.form_at(ReadFixture::offset_of("(* w h)"));
        assert_eq!(
            form.map(|form| form.to_string()).as_deref(),
            Some("(* w h)")
        );
    }

    #[test]
    fn a_form_read_alone_keeps_the_spans_of_a_whole_file_read() {
        let fixture = ReadFixture::new();
        let whole = fixture.whole_file();
        let second = &whole[1];
        let alone = fixture
            .form_at(second.span().range().start)
            .expect("the form reads");
        assert_eq!(format!("{alone:?}"), format!("{second:?}"));
    }

    #[test]
    fn an_offset_that_starts_no_form_reads_nothing() {
        let fixture = ReadFixture::new();
        for text in [" (w h)", ") (* w", "; a note", "\n(defn half"] {
            assert!(
                fixture.form_at(ReadFixture::offset_of(text)).is_none(),
                "{text:?}"
            );
        }
        assert!(fixture.form_at(ReadFixture::TEXT.len()).is_none());
    }

    #[test]
    fn an_offset_outside_the_excerpt_reads_nothing() {
        let text = "(defn area (w h) (* w h))\n(area 2 3)\n";
        let mut sources = SourceMap::default();
        let module = sources.add("geometry.knv", text);
        let inside = sources
            .add_inside(module, TextPosition::new(2, 1), "(area 2 3)")
            .expect("the forms sit at that position");
        assert!(Reader::read_form_at(sources.file(inside), 0).is_none());
    }

    #[test]
    fn a_form_with_a_read_error_reads_nothing() {
        let mut sources = SourceMap::default();
        let file = sources.add("broken.knv", "(area [2] 3)");
        assert!(Reader::read_form_at(sources.file(file), 0).is_none());
    }

    #[test]
    fn a_source_inside_another_reads_only_its_own_forms_at_their_offsets() {
        let text = "(defn area (w h) (* w h))\n(area 2 3)\n(area 4 5)\n";
        let mut sources = SourceMap::default();
        let module = sources.add("geometry.knv", text);
        let inside = sources
            .add_inside(module, TextPosition::new(2, 1), "(area 2 3)")
            .expect("the forms sit at that position");
        let forms = Reader::new(sources.file(inside))
            .read()
            .unwrap_or_else(|_| panic!("the forms read"));
        let start = text.find("(area 2").expect("the text holds the call");
        let starts: Vec<usize> = forms.iter().map(|form| form.span().range().start).collect();
        assert_eq!(starts, [start]);
    }
}
