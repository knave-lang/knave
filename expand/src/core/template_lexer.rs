use super::lexeme::Lexeme;

#[derive(Clone, Copy, Default)]
enum Context {
    #[default]
    Code,
    Quoted,
    Raw(usize),
    LineComment,
    BlockComment(usize),
}

#[derive(Default)]
pub struct TemplateLexer {
    context: Context,
}

impl TemplateLexer {
    const UNDERSCORE: char = '_';
    const DECIMAL_POINT: char = '.';
    const QUOTE: char = '"';
    const APOSTROPHE: char = '\'';
    const ESCAPE: char = '\\';
    const RAW_HASH: char = '#';
    const UNICODE_ESCAPE: char = 'u';
    const UNICODE_END: char = '}';
    const LINE_END: char = '\n';
    const LINE_COMMENT: &'static str = "//";
    const BLOCK_COMMENT_START: &'static str = "/*";
    const BLOCK_COMMENT_END: &'static str = "*/";
    const STRING_PREFIXES: [&'static str; 2] = ["b", "c"];
    const RAW_PREFIXES: [&'static str; 3] = ["r", "br", "cr"];
    const BYTE_PREFIX: &'static str = "b";

    pub fn digits(number: &str) -> String {
        number.replace(Self::UNDERSCORE, "")
    }

    pub fn in_code(&self) -> bool {
        matches!(self.context, Context::Code)
    }

    pub fn lex<'t>(&mut self, literal: &'t str) -> Vec<Lexeme<'t>> {
        let mut lexemes = Vec::new();
        let mut rest = literal;
        while !rest.is_empty() {
            rest = self.advance(rest, &mut lexemes);
        }
        lexemes
    }

    fn advance<'t>(&mut self, rest: &'t str, lexemes: &mut Vec<Lexeme<'t>>) -> &'t str {
        let ended = match self.context {
            Context::Code => return self.past_code(rest, lexemes),
            Context::Quoted => Self::quote_end(rest),
            Context::Raw(hashes) => Self::raw_end(rest, hashes),
            Context::LineComment => rest
                .find(Self::LINE_END)
                .map(|end| end + Self::LINE_END.len_utf8()),
            Context::BlockComment(depth) => return self.past_block_comment(rest, depth),
        };
        match ended {
            Some(end) => {
                self.context = Context::Code;
                &rest[end..]
            }
            None => "",
        }
    }

    fn past_code<'t>(&mut self, rest: &'t str, lexemes: &mut Vec<Lexeme<'t>>) -> &'t str {
        let code = rest.trim_start();
        if code.is_empty() {
            return code;
        }
        let (lexeme, length) = self.next_lexeme(code);
        lexemes.extend(lexeme);
        &code[length..]
    }

    fn next_lexeme<'t>(&mut self, rest: &'t str) -> (Option<Lexeme<'t>>, usize) {
        if let Some((context, length)) = Self::opening(rest) {
            self.context = context;
            let lexeme =
                matches!(context, Context::Quoted | Context::Raw(_)).then_some(Lexeme::Text);
            return (lexeme, length);
        }
        if let Some(length) = Self::character_length(rest) {
            return (Some(Lexeme::Text), length);
        }
        let first = rest.chars().next().unwrap_or_default();
        match first {
            digit if digit.is_ascii_digit() => {
                Self::spanning(rest, Self::number_length(rest), Lexeme::Number)
            }
            letter if Self::starts_word(letter) => {
                Self::spanning(rest, Self::alphanumeric_length(rest), Lexeme::Word)
            }
            symbol => (Some(Lexeme::Symbol(symbol)), symbol.len_utf8()),
        }
    }

    fn opening(rest: &str) -> Option<(Context, usize)> {
        if rest.starts_with(Self::LINE_COMMENT) {
            return Some((Context::LineComment, Self::LINE_COMMENT.len()));
        }
        if rest.starts_with(Self::BLOCK_COMMENT_START) {
            return Some((Context::BlockComment(1), Self::BLOCK_COMMENT_START.len()));
        }
        if let Some(raw) = Self::raw_opening(rest) {
            return Some(raw);
        }
        let prefix = Self::STRING_PREFIXES
            .iter()
            .find(|prefix| {
                rest.strip_prefix(*prefix)
                    .is_some_and(|after| after.starts_with(Self::QUOTE))
            })
            .map_or(0, |prefix| prefix.len());
        rest[prefix..]
            .starts_with(Self::QUOTE)
            .then_some((Context::Quoted, prefix + Self::QUOTE.len_utf8()))
    }

    fn raw_opening(rest: &str) -> Option<(Context, usize)> {
        Self::RAW_PREFIXES.iter().find_map(|prefix| {
            let after = rest.strip_prefix(prefix)?;
            let hashes = after.len() - after.trim_start_matches(Self::RAW_HASH).len();
            after[hashes..]
                .starts_with(Self::QUOTE)
                .then_some((Context::Raw(hashes), prefix.len() + hashes + 1))
        })
    }

    fn character_length(rest: &str) -> Option<usize> {
        let prefix = if rest.starts_with(Self::BYTE_PREFIX) {
            Self::BYTE_PREFIX.len()
        } else {
            0
        };
        let inside = rest[prefix..].strip_prefix(Self::APOSTROPHE)?;
        let content = Self::character_content_length(inside)?;
        inside[content..]
            .starts_with(Self::APOSTROPHE)
            .then_some(prefix + 2 * Self::APOSTROPHE.len_utf8() + content)
    }

    fn character_content_length(inside: &str) -> Option<usize> {
        let mut characters = inside.char_indices();
        let (_, first) = characters.next()?;
        match first {
            Self::ESCAPE => match characters.next()? {
                (_, Self::UNICODE_ESCAPE) => inside
                    .find(Self::UNICODE_END)
                    .map(|end| end + Self::UNICODE_END.len_utf8()),
                (position, escaped) => Some(position + escaped.len_utf8()),
            },
            other => Some(other.len_utf8()),
        }
    }

    fn quote_end(rest: &str) -> Option<usize> {
        let mut characters = rest.char_indices();
        while let Some((position, character)) = characters.next() {
            match character {
                Self::ESCAPE => {
                    characters.next();
                }
                Self::QUOTE => return Some(position + character.len_utf8()),
                _ => {}
            }
        }
        None
    }

    fn raw_end(rest: &str, hashes: usize) -> Option<usize> {
        let closing = format!(
            "{}{}",
            Self::QUOTE,
            Self::RAW_HASH.to_string().repeat(hashes)
        );
        rest.find(&closing).map(|end| end + closing.len())
    }

    fn past_block_comment<'t>(&mut self, rest: &'t str, depth: usize) -> &'t str {
        let start = rest.find(Self::BLOCK_COMMENT_START);
        let end = rest.find(Self::BLOCK_COMMENT_END);
        match (start, end) {
            (Some(start), Some(end)) if start < end => {
                self.context = Context::BlockComment(depth + 1);
                &rest[start + Self::BLOCK_COMMENT_START.len()..]
            }
            (_, Some(end)) => {
                self.context = match depth - 1 {
                    0 => Context::Code,
                    outer => Context::BlockComment(outer),
                };
                &rest[end + Self::BLOCK_COMMENT_END.len()..]
            }
            (Some(start), None) => {
                self.context = Context::BlockComment(depth + 1);
                &rest[start + Self::BLOCK_COMMENT_START.len()..]
            }
            (None, None) => "",
        }
    }

    fn spanning<'t>(
        rest: &'t str,
        length: usize,
        lexeme: fn(&'t str) -> Lexeme<'t>,
    ) -> (Option<Lexeme<'t>>, usize) {
        (Some(lexeme(&rest[..length])), length)
    }

    fn number_length(rest: &str) -> usize {
        let whole = Self::alphanumeric_length(rest);
        match rest[whole..].strip_prefix(Self::DECIMAL_POINT) {
            Some(fraction) if !fraction.starts_with(Self::starts_word) => {
                whole + Self::DECIMAL_POINT.len_utf8() + Self::alphanumeric_length(fraction)
            }
            _ => whole,
        }
    }

    fn alphanumeric_length(text: &str) -> usize {
        text.find(|character: char| !Self::continues_word(character))
            .unwrap_or(text.len())
    }

    fn starts_word(character: char) -> bool {
        character.is_alphabetic()
    }

    fn continues_word(character: char) -> bool {
        character.is_alphanumeric() || character == Self::UNDERSCORE
    }
}

#[cfg(test)]
mod tests {
    use super::TemplateLexer;
    use crate::core::lexeme::Lexeme;

    struct LexerFixture;

    impl LexerFixture {
        fn lexed(text: &str) -> Vec<Lexeme<'_>> {
            TemplateLexer::default().lex(text)
        }
    }

    #[test]
    fn skips_a_line_comment_and_the_quote_in_it() {
        assert_eq!(
            LexerFixture::lexed("a // \" b\nc"),
            [Lexeme::Word("a"), Lexeme::Word("c")]
        );
    }

    #[test]
    fn skips_a_nested_block_comment_and_the_quote_in_it() {
        assert_eq!(
            LexerFixture::lexed("a /* x /* \" */ y */ b"),
            [Lexeme::Word("a"), Lexeme::Word("b")]
        );
    }

    #[test]
    fn reads_a_char_literal_that_holds_a_quote_as_one_lexeme() {
        assert_eq!(
            LexerFixture::lexed("'\"' + x"),
            [Lexeme::Text, Lexeme::Symbol('+'), Lexeme::Word("x")]
        );
    }

    #[test]
    fn reads_a_byte_literal_that_holds_a_quote_as_one_lexeme() {
        assert_eq!(
            LexerFixture::lexed("b'\"' x"),
            [Lexeme::Text, Lexeme::Word("x")]
        );
    }

    #[test]
    fn reads_an_escaped_char_literal_as_one_lexeme() {
        for text in ["'\\'' x", "'\\\\' x", "'\\u{22}' x"] {
            assert_eq!(
                LexerFixture::lexed(text),
                [Lexeme::Text, Lexeme::Word("x")],
                "{text}"
            );
        }
    }

    #[test]
    fn reads_a_lifetime_as_an_apostrophe_and_a_word() {
        assert_eq!(
            LexerFixture::lexed("&'a str"),
            [
                Lexeme::Symbol('&'),
                Lexeme::Symbol('\''),
                Lexeme::Word("a"),
                Lexeme::Word("str")
            ]
        );
    }

    #[test]
    fn reads_raw_strings_that_hold_a_quote_as_one_lexeme() {
        for text in ["r\"\\\" x", "r#\"a\"b\"# x", "br\"\\\" x", "br#\"\"\"# x"] {
            assert_eq!(
                LexerFixture::lexed(text),
                [Lexeme::Text, Lexeme::Word("x")],
                "{text}"
            );
        }
    }

    #[test]
    fn reads_a_string_with_an_escaped_quote_as_one_lexeme() {
        assert_eq!(
            LexerFixture::lexed("\"a\\\"b\" x"),
            [Lexeme::Text, Lexeme::Word("x")]
        );
    }

    #[test]
    fn keeps_a_string_open_across_two_literals() {
        let mut lexer = TemplateLexer::default();
        assert_eq!(lexer.lex("(\"a"), [Lexeme::Symbol('('), Lexeme::Text]);
        assert!(!lexer.in_code());
        assert_eq!(
            lexer.lex("b\", c)"),
            [Lexeme::Symbol(','), Lexeme::Word("c"), Lexeme::Symbol(')')]
        );
        assert!(lexer.in_code());
    }
}
