use std::ops::Range;

pub struct DocstringLines {
    lines: Vec<String>,
    written_ranges: Vec<Range<usize>>,
}

impl DocstringLines {
    const QUOTE: char = '"';
    const ESCAPE: char = '\\';
    const ESCAPED: [char; 2] = [Self::QUOTE, Self::ESCAPE];

    pub fn of_token(token: &str) -> Option<Self> {
        let contents = token.strip_prefix(Self::QUOTE)?.strip_suffix(Self::QUOTE)?;
        let mut lines = Vec::new();
        let mut written_ranges = Vec::new();
        let mut start = Self::QUOTE.len_utf8();
        for written in contents.split('\n') {
            lines.push(Self::unescaped(written)?);
            written_ranges.push(start..start + written.len());
            start += written.len() + 1;
        }
        Some(Self {
            lines,
            written_ranges,
        })
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn written_range(&self, line: usize) -> Range<usize> {
        let range = self.written_ranges[line].clone();
        let indentation = self.lines[line].len() - self.lines[line].trim_start().len();
        range.start + indentation..range.end
    }

    pub fn token_of(lines: &[String]) -> String {
        let mut token = String::from(Self::QUOTE);
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                token.push('\n');
            }
            for character in line.chars() {
                if Self::ESCAPED.contains(&character) {
                    token.push(Self::ESCAPE);
                }
                token.push(character);
            }
        }
        token.push(Self::QUOTE);
        token
    }

    fn unescaped(written: &str) -> Option<String> {
        let mut line = String::with_capacity(written.len());
        let mut characters = written.chars();
        while let Some(character) = characters.next() {
            if character == Self::ESCAPE {
                line.push(
                    characters
                        .next()
                        .filter(|next| Self::ESCAPED.contains(next))?,
                );
            } else {
                line.push(character);
            }
        }
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::DocstringLines;

    #[test]
    fn a_token_reads_to_lines_without_its_quote_and_backslash_escapes() {
        let lines =
            DocstringLines::of_token("\"Pads.\n  (pad \\\"a\\\\b\\\")\"").expect("it reads");
        assert_eq!(lines.lines(), ["Pads.", "  (pad \"a\\b\")"]);
    }

    #[test]
    fn a_token_with_another_escape_reads_to_nothing() {
        assert!(DocstringLines::of_token("\"Tabs.\\t\"").is_none());
    }

    #[test]
    fn lines_write_back_to_the_token_they_read_from() {
        let token = "\"Pads.\n  (pad \\\"a\\\\b\\\")\"";
        let lines = DocstringLines::of_token(token).expect("it reads");
        assert_eq!(DocstringLines::token_of(lines.lines()), token);
    }

    #[test]
    fn a_written_range_covers_a_line_after_its_indentation() {
        let token = "\"Pads.\n  (pad \\\"a\\\")\"";
        let lines = DocstringLines::of_token(token).expect("it reads");
        assert_eq!(&token[lines.written_range(1)], "(pad \\\"a\\\")");
    }
}
