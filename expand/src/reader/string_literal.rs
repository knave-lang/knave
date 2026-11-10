use std::ops::Range;

use super::cursor::Cursor;
use super::syntax::Syntax;

pub enum StringProblem {
    UnknownEscape(Range<usize>),
    Unterminated,
}

pub struct StringLiteral {
    pub value: String,
    pub problems: Vec<StringProblem>,
}

impl StringLiteral {
    const QUOTE: char = '"';

    pub fn quoted(text: &str) -> String {
        let mut quoted = String::from(Self::QUOTE);
        for character in text.chars() {
            match Self::escape_of(character) {
                Some(escape) => {
                    quoted.push(Syntax::ESCAPE);
                    quoted.push(escape);
                }
                None => quoted.push(character),
            }
        }
        quoted.push(Self::QUOTE);
        quoted
    }

    fn escape_of(character: char) -> Option<char> {
        StringLiteralReader::SIMPLE_ESCAPES
            .iter()
            .find(|(_, escaped)| *escaped == character)
            .map(|(escape, _)| *escape)
    }
}

pub struct StringLiteralReader<'c, 'a> {
    cursor: &'c mut Cursor<'a>,
    value: String,
    problems: Vec<StringProblem>,
}

impl<'c, 'a> StringLiteralReader<'c, 'a> {
    const SIMPLE_ESCAPES: [(char, char); 7] = [
        ('n', '\n'),
        ('r', '\r'),
        ('t', '\t'),
        ('0', '\0'),
        ('\\', '\\'),
        ('"', '"'),
        ('\'', '\''),
    ];
    const UNICODE_ESCAPE: char = 'u';
    const UNICODE_OPEN: char = '{';
    const UNICODE_CLOSE: char = '}';
    const HEX_RADIX: u32 = 16;

    pub fn read(cursor: &'c mut Cursor<'a>) -> StringLiteral {
        let mut reader = Self {
            cursor,
            value: String::new(),
            problems: Vec::new(),
        };
        reader.cursor.advance();
        reader.read_contents();
        StringLiteral {
            value: reader.value,
            problems: reader.problems,
        }
    }

    pub fn read_written_ranges(cursor: &'c mut Cursor<'a>) -> (String, Vec<Range<usize>>) {
        let mut reader = Self {
            cursor,
            value: String::new(),
            problems: Vec::new(),
        };
        reader.cursor.advance();
        let written_ranges = reader.read_contents_recording_written_ranges();
        (reader.value, written_ranges)
    }

    fn read_contents_recording_written_ranges(&mut self) -> Vec<Range<usize>> {
        let mut written_ranges = Vec::new();
        loop {
            let start = self.cursor.position();
            match self.cursor.advance() {
                None | Some(Syntax::QUOTE) => return written_ranges,
                Some(Syntax::ESCAPE) => self.read_escape(),
                Some(character) => self.value.push(character),
            }
            written_ranges.resize(self.value.len(), start..self.cursor.position());
        }
    }

    fn read_contents(&mut self) {
        loop {
            match self.cursor.advance() {
                None => return self.problems.push(StringProblem::Unterminated),
                Some(Syntax::QUOTE) => return,
                Some(Syntax::ESCAPE) => self.read_escape(),
                Some(character) => self.value.push(character),
            }
        }
    }

    fn read_escape(&mut self) {
        let start = self.cursor.position() - Syntax::ESCAPE.len_utf8();
        match self.escaped_character() {
            Some(character) => self.value.push(character),
            None => self
                .problems
                .push(StringProblem::UnknownEscape(start..self.cursor.position())),
        }
    }

    fn escaped_character(&mut self) -> Option<char> {
        let marker = self.cursor.advance()?;
        if marker == Self::UNICODE_ESCAPE {
            return self.read_unicode_escape();
        }
        Self::SIMPLE_ESCAPES
            .iter()
            .find(|(escape, _)| *escape == marker)
            .map(|(_, character)| *character)
    }

    fn read_unicode_escape(&mut self) -> Option<char> {
        self.expect(Self::UNICODE_OPEN)?;
        let digits = self
            .cursor
            .advance_while(|character| character.is_ascii_hexdigit());
        self.expect(Self::UNICODE_CLOSE)?;
        let code_point = u32::from_str_radix(digits, Self::HEX_RADIX).ok()?;
        char::from_u32(code_point)
    }

    fn expect(&mut self, expected: char) -> Option<()> {
        (self.cursor.peek() == Some(expected)).then(|| {
            self.cursor.advance();
        })
    }
}

#[cfg(test)]
mod tests {
    use super::StringLiteral;
    use crate::reader::{FormKind, Reader};
    use crate::source::SourceMap;

    struct QuotingFixture;

    impl QuotingFixture {
        fn read_back(text: &str) -> String {
            let mut sources = SourceMap::default();
            let source = sources.add("quoted.knv", StringLiteral::quoted(text));
            let forms = Reader::new(sources.file(source))
                .read()
                .expect("a quoted string reads");
            match forms[0].kind() {
                FormKind::String(read) => read.to_string(),
                _ => panic!("a quoted string reads as a string"),
            }
        }
    }

    #[test]
    fn a_quoted_string_reads_back_as_written() {
        let text = "a \"b\" \\c\nd\te'f";
        assert_eq!(QuotingFixture::read_back(text), text);
    }
}
