use super::open_delimiter::OpenDelimiter;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Associativity {
    Left,
    Associative,
    NonAssociative,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RustOperator {
    symbol: &'static str,
    precedence: u8,
    associativity: Associativity,
}

impl RustOperator {
    const TABLE: [(&'static str, u8, Associativity); 24] = [
        ("=", 0, Associativity::NonAssociative),
        ("+=", 0, Associativity::NonAssociative),
        ("-=", 0, Associativity::NonAssociative),
        ("*=", 0, Associativity::NonAssociative),
        ("/=", 0, Associativity::NonAssociative),
        ("%=", 0, Associativity::NonAssociative),
        ("||", 1, Associativity::Associative),
        ("&&", 2, Associativity::Associative),
        ("==", 3, Associativity::NonAssociative),
        ("!=", 3, Associativity::NonAssociative),
        ("<", 3, Associativity::NonAssociative),
        (">", 3, Associativity::NonAssociative),
        ("<=", 3, Associativity::NonAssociative),
        (">=", 3, Associativity::NonAssociative),
        ("|", 4, Associativity::Left),
        ("^", 5, Associativity::Left),
        ("&", 6, Associativity::Left),
        ("<<", 7, Associativity::Left),
        (">>", 7, Associativity::Left),
        ("+", 8, Associativity::Left),
        ("-", 8, Associativity::Left),
        ("*", 9, Associativity::Left),
        ("/", 9, Associativity::Left),
        ("%", 9, Associativity::Left),
    ];

    pub fn and() -> Self {
        Self::at("&&").expect("the table holds and")
    }

    pub fn or() -> Self {
        Self::at("||").expect("the table holds or")
    }

    pub fn symbol(self) -> &'static str {
        self.symbol
    }

    pub fn at(token: &str) -> Option<Self> {
        Self::TABLE
            .iter()
            .find(|(symbol, ..)| *symbol == token)
            .map(|(symbol, precedence, associativity)| Self {
                symbol,
                precedence: *precedence,
                associativity: *associativity,
            })
    }

    pub fn outermost(text: &str) -> Option<Self> {
        RustText::new(text)
            .tokens()
            .into_iter()
            .filter_map(Self::at)
            .min_by_key(|operator| operator.precedence)
    }

    pub fn needs_parentheses(self, outer: Self, side: Side) -> bool {
        if self.precedence > outer.precedence {
            return false;
        }
        if self.precedence < outer.precedence {
            return true;
        }
        match outer.associativity {
            Associativity::Associative => false,
            Associativity::Left => side == Side::Right,
            Associativity::NonAssociative => true,
        }
    }
}

pub struct RustText<'a> {
    text: &'a str,
}

impl<'a> RustText<'a> {
    pub const ARGUMENT_ENDS: [char; 3] = [',', ')', ']'];

    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    pub fn has_top_level_operator(&self) -> bool {
        self.tokens().len() > 1
    }

    pub fn tokens(&self) -> Vec<&'a str> {
        TokenScanner::scan(self.text).tokens
    }

    pub fn closes_every_delimiter(&self) -> bool {
        TokenScanner::scan(self.text).open.is_empty()
    }

    pub fn argument_start(&self) -> Option<ArgumentStart<'a>> {
        let mut scanner = TokenScanner::scan(self.text);
        if scanner.literal != Literal::Outside {
            return None;
        }
        let open = scanner.open.pop()?;
        let written = self.text.trim_end();
        if written.ends_with(',') {
            return Some(ArgumentStart { open, name: None });
        }
        let name = Self::argument_name(written)?;
        Some(ArgumentStart {
            open,
            name: Some(name),
        })
    }

    pub fn trailing_identifier(text: &str) -> &str {
        let before = text
            .trim_end_matches(|character: char| character.is_alphanumeric() || character == '_');
        &text[before.len()..]
    }

    fn argument_name(written: &'a str) -> Option<&'a str> {
        let named = written.strip_suffix('=')?.trim_end();
        let name = Self::trailing_identifier(named);
        let before_name = &named[..named.len() - name.len()];
        (!name.is_empty() && before_name.trim_end().ends_with(',')).then_some(name)
    }
}

pub struct ArgumentStart<'a> {
    open: OpenDelimiter<'a>,
    name: Option<&'a str>,
}

impl<'a> ArgumentStart<'a> {
    pub fn open(&self) -> &OpenDelimiter<'a> {
        &self.open
    }

    pub fn name(&self) -> Option<&'a str> {
        self.name
    }
}

struct TokenScanner<'a> {
    text: &'a str,
    tokens: Vec<&'a str>,
    start: Option<usize>,
    open: Vec<OpenDelimiter<'a>>,
    literal: Literal,
    literal_start: usize,
    escaped: bool,
}

impl<'a> TokenScanner<'a> {
    const QUOTE_LENGTH: usize = 1;

    fn scan(text: &'a str) -> Self {
        let mut scanner = Self {
            text,
            tokens: Vec::new(),
            start: None,
            open: Vec::new(),
            literal: Literal::Outside,
            literal_start: 0,
            escaped: false,
        };
        let mut characters = text.char_indices().peekable();
        while let Some((position, character)) = characters.next() {
            if scanner.consume_literal(position, character) {
                continue;
            }
            if scanner.consume_range(position, character, &mut characters) {
                continue;
            }
            if scanner.consume_delimiter(position, character) {
                continue;
            }
            if character.is_whitespace() && scanner.open.is_empty() {
                scanner.flush(position);
            } else {
                scanner.start.get_or_insert(position);
            }
        }
        scanner.flush(text.len());
        scanner
    }

    fn consume_literal(&mut self, position: usize, character: char) -> bool {
        if self.literal == Literal::Outside {
            return false;
        }
        if self.escaped {
            self.escaped = false;
        } else if character == '\\' {
            self.escaped = true;
        } else if self.literal.closes(character) {
            self.close_literal(position);
        }
        true
    }

    fn close_literal(&mut self, end: usize) {
        if let (Literal::String, Some(open)) = (self.literal, self.open.last_mut()) {
            open.hold_text(&self.text[self.literal_start + 1..end]);
        }
        self.literal = Literal::Outside;
    }

    fn opens_character(&self, position: usize) -> bool {
        let after_quote = &self.text[position + Self::QUOTE_LENGTH..];
        after_quote.starts_with('\\') || after_quote.chars().nth(1) == Some('\'')
    }

    fn consume_range(
        &mut self,
        position: usize,
        character: char,
        characters: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
    ) -> bool {
        if character != '.' || !self.open.is_empty() {
            return false;
        }
        if characters.peek().is_none_or(|(_, next)| *next != '.') {
            return false;
        }
        self.flush(position);
        characters.next();
        self.tokens.push("..");
        true
    }

    fn consume_delimiter(&mut self, position: usize, character: char) -> bool {
        match character {
            '"' => {
                self.literal = Literal::String;
                self.literal_start = position;
            }
            '\'' if self.opens_character(position) => self.literal = Literal::Character,
            '(' => self.open(self.macro_called_at(position)),
            '[' | '{' => self.open(None),
            ')' | ']' | '}' => {
                self.open.pop();
            }
            ',' => return self.count_argument(),
            _ => return false,
        }
        self.start.get_or_insert(position);
        true
    }

    fn open(&mut self, callee: Option<&'a str>) {
        self.open.push(OpenDelimiter::new(callee));
    }

    fn count_argument(&mut self) -> bool {
        if let Some(open) = self.open.last_mut() {
            open.count_argument();
        }
        false
    }

    fn macro_called_at(&self, position: usize) -> Option<&'a str> {
        let path = self.text[..position].trim_end().strip_suffix('!')?;
        Some(RustText::trailing_identifier(path))
    }

    fn flush(&mut self, position: usize) {
        if let Some(start) = self.start.take() {
            self.tokens.push(&self.text[start..position]);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Literal {
    Outside,
    String,
    Character,
}

impl Literal {
    fn closes(self, character: char) -> bool {
        match self {
            Literal::Outside => false,
            Literal::String => character == '"',
            Literal::Character => character == '\'',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RustText;

    struct TokenCase;

    impl TokenCase {
        fn tokens(text: &str) -> Vec<&str> {
            RustText::new(text).tokens()
        }
    }

    #[test]
    fn a_quote_character_is_one_token() {
        assert_eq!(TokenCase::tokens(r#"'"' == c"#), [r#"'"'"#, "==", "c"]);
    }

    #[test]
    fn an_escaped_quote_character_is_one_token() {
        assert_eq!(TokenCase::tokens(r#"'\"' == c"#), [r#"'\"'"#, "==", "c"]);
    }

    #[test]
    fn a_lifetime_opens_no_literal() {
        assert_eq!(
            TokenCase::tokens("f::<&'static str>(x) + 1"),
            ["f::<&'static", "str>(x)", "+", "1"]
        );
    }

    #[test]
    fn a_lone_quote_at_the_end_opens_no_literal() {
        assert_eq!(TokenCase::tokens("x + y'"), ["x", "+", "y'"]);
    }
}
