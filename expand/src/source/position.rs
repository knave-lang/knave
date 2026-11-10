use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextPosition {
    line: usize,
    column: usize,
}

impl TextPosition {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub fn of_offset(text: &str, offset: usize) -> Self {
        LineStarts::of(&text[..offset]).position(offset)
    }

    pub fn line(self) -> usize {
        self.line
    }

    pub fn column(self) -> usize {
        self.column
    }

    pub fn offset_in(self, text: &str) -> Option<usize> {
        let line_start = self.line_start_in(text)?;
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |length| line_start + length);
        let offset = line_start + self.column.checked_sub(1)?;
        (offset <= line_end && text.is_char_boundary(offset)).then_some(offset)
    }

    fn line_start_in(self, text: &str) -> Option<usize> {
        LineStarts::starts_in(text).nth(self.line.checked_sub(1)?)
    }
}

impl fmt::Display for TextPosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

pub struct LineStarts {
    starts: Vec<u32>,
}

impl LineStarts {
    pub fn of(text: &str) -> Self {
        let mut starts = Vec::with_capacity(text.matches('\n').count() + 1);
        starts.extend(Self::starts_in(text).map(Self::stored));
        Self { starts }
    }

    pub fn position(&self, offset: usize) -> TextPosition {
        let lines_started = self
            .starts
            .partition_point(|start| *start as usize <= offset);
        let line_start = self.starts[lines_started - 1] as usize;
        TextPosition::new(lines_started, offset - line_start + 1)
    }

    fn starts_in(text: &str) -> impl Iterator<Item = usize> + '_ {
        let following_newlines = text.match_indices('\n').map(|(offset, _)| offset + 1);
        std::iter::once(0).chain(following_newlines)
    }

    fn stored(offset: usize) -> u32 {
        u32::try_from(offset).expect("source files are smaller than 4 GiB")
    }
}

#[cfg(test)]
mod tests {
    use super::{LineStarts, TextPosition};

    struct PositionFixture;

    impl PositionFixture {
        const TEXT: &str = "(defn main ()\n  (prn \"☃\" 1))\n";

        fn offset(line: usize, column: usize) -> Option<usize> {
            TextPosition::new(line, column).offset_in(Self::TEXT)
        }
    }

    #[test]
    fn the_first_position_is_the_start_of_the_text() {
        assert_eq!(PositionFixture::offset(1, 1), Some(0));
    }

    #[test]
    fn a_column_counts_bytes_from_the_start_of_its_line() {
        assert_eq!(
            PositionFixture::offset(2, 3),
            PositionFixture::TEXT.find("(prn")
        );
        assert_eq!(
            PositionFixture::offset(2, 13),
            PositionFixture::TEXT.find(" 1")
        );
    }

    #[test]
    fn a_column_may_sit_at_the_end_of_its_line() {
        assert_eq!(
            PositionFixture::offset(1, 14),
            PositionFixture::TEXT.find('\n')
        );
    }

    #[test]
    fn a_column_past_the_end_of_its_line_has_no_offset() {
        assert_eq!(PositionFixture::offset(1, 15), None);
    }

    #[test]
    fn a_column_inside_a_character_has_no_offset() {
        assert_eq!(PositionFixture::offset(2, 10), None);
    }

    #[test]
    fn a_line_past_the_end_of_the_text_has_no_offset() {
        assert_eq!(PositionFixture::offset(4, 1), None);
    }

    #[test]
    fn line_and_column_zero_have_no_offset() {
        assert_eq!(PositionFixture::offset(0, 1), None);
        assert_eq!(PositionFixture::offset(1, 0), None);
    }

    #[test]
    fn an_offset_on_the_first_line_counts_its_column_from_one() {
        let position = TextPosition::of_offset(PositionFixture::TEXT, 6);
        assert_eq!((position.line(), position.column()), (1, 7));
    }

    #[test]
    fn an_offset_after_a_multibyte_character_counts_its_column_in_bytes() {
        let offset = PositionFixture::TEXT
            .find(" 1")
            .expect("the text holds a 1");
        let position = TextPosition::of_offset(PositionFixture::TEXT, offset);
        assert_eq!(position, TextPosition::new(2, 13));
    }

    #[test]
    fn the_offset_of_a_position_gives_the_position_back() {
        let position = TextPosition::new(2, 3);
        let offset = position
            .offset_in(PositionFixture::TEXT)
            .expect("the position is in the text");
        assert_eq!(
            TextPosition::of_offset(PositionFixture::TEXT, offset),
            position
        );
    }

    struct LineStartsFixture;

    impl LineStartsFixture {
        const TEXT: &str = "(defn main ()\n\n  (prn 1))\n";

        fn position(offset: usize) -> TextPosition {
            LineStarts::of(Self::TEXT).position(offset)
        }
    }

    #[test]
    fn the_first_offset_is_the_first_column_of_the_first_line() {
        assert_eq!(LineStartsFixture::position(0), TextPosition::new(1, 1));
    }

    #[test]
    fn a_newline_ends_its_own_line() {
        let newline = LineStartsFixture::TEXT.find('\n').expect("a newline");
        assert_eq!(
            LineStartsFixture::position(newline),
            TextPosition::new(1, 14)
        );
    }

    #[test]
    fn an_empty_line_counts_as_a_line() {
        let call = LineStartsFixture::TEXT.find("(prn").expect("a call");
        assert_eq!(LineStartsFixture::position(call), TextPosition::new(3, 3));
    }

    #[test]
    fn the_end_of_the_text_starts_a_line_after_a_final_newline() {
        let end = LineStartsFixture::TEXT.len();
        assert_eq!(LineStartsFixture::position(end), TextPosition::new(4, 1));
    }
}
