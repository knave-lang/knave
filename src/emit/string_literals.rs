pub struct StringLiterals;

impl StringLiterals {
    const QUOTE: char = '"';
    const ESCAPE: char = '\\';
    const QUOTED_CHARACTER: &'static str = "'\"'";

    pub fn code_spans(text: &str) -> Vec<&str> {
        let mut spans = Vec::new();
        let mut rest = text;
        while let Some(start) = Self::string_start(rest) {
            spans.push(&rest[..start]);
            rest = &rest[Self::string_end(rest, start)..];
        }
        spans.push(rest);
        spans
    }

    fn string_start(text: &str) -> Option<usize> {
        text.match_indices(Self::QUOTE)
            .map(|(index, _)| index)
            .find(|&index| !Self::quotes_a_character(text, index))
    }

    fn quotes_a_character(text: &str, index: usize) -> bool {
        index > 0 && text[index - 1..].starts_with(Self::QUOTED_CHARACTER)
    }

    fn string_end(text: &str, start: usize) -> usize {
        let contents_start = start + Self::QUOTE.len_utf8();
        let mut escaped = false;
        for (offset, character) in text[contents_start..].char_indices() {
            if escaped {
                escaped = false;
            } else if character == Self::ESCAPE {
                escaped = true;
            } else if character == Self::QUOTE {
                return contents_start + offset + Self::QUOTE.len_utf8();
            }
        }
        text.len()
    }
}

#[cfg(test)]
mod tests {
    use super::StringLiterals;

    #[test]
    fn code_without_a_string_is_one_span() {
        assert_eq!(StringLiterals::code_spans("f(x)"), ["f(x)"]);
    }

    #[test]
    fn a_string_splits_the_code_around_it() {
        assert_eq!(StringLiterals::code_spans(r#"f("a b", c)"#), ["f(", ", c)"]);
    }

    #[test]
    fn an_escaped_quote_stays_inside_its_string() {
        assert_eq!(
            StringLiterals::code_spans(r#"f("a \" b", c)"#),
            ["f(", ", c)"]
        );
    }

    #[test]
    fn a_quote_character_literal_is_code() {
        assert_eq!(StringLiterals::code_spans(r#"f('"', c)"#), [r#"f('"', c)"#]);
    }

    #[test]
    fn an_unclosed_string_ends_the_code() {
        assert_eq!(StringLiterals::code_spans(r#"f("open"#), ["f(", ""]);
    }
}
