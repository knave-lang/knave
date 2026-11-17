use super::Reader;
use super::cursor::Cursor;
use super::string_literal::StringLiteralReader;
use super::syntax::Syntax;
use crate::source::SourceFile;

pub struct ListClose<'a> {
    reader: Reader<'a>,
    unclosed: usize,
}

impl<'a> ListClose<'a> {
    pub fn after(source: &'a SourceFile, offset: usize) -> usize {
        let text = &source.text()[..source.excerpt_range().end];
        let reader = Reader {
            cursor: Cursor::starting_at(text, offset),
            ..Reader::new(source)
        };
        let mut scan = Self {
            reader,
            unclosed: 0,
        };
        scan.close_position()
    }

    fn close_position(&mut self) -> usize {
        while let Some(next) = self.reader.skip_trivia() {
            match next {
                Syntax::CLOSE if self.unclosed == 0 => return self.reader.cursor.position(),
                Syntax::OPEN | Syntax::CLOSE => {
                    self.unclosed = self.reader.skip_list_part(next, self.unclosed);
                }
                Syntax::QUOTE => {
                    StringLiteralReader::read(&mut self.reader.cursor);
                }
                _ => self.skip_atom(),
            }
        }
        self.reader.cursor.position()
    }

    fn skip_atom(&mut self) {
        if self.reader.cursor.advance_atom().is_empty() {
            self.reader.cursor.advance();
        }
    }
}
