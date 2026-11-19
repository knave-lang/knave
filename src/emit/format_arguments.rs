use crate::core::Piece;

pub struct FormatArguments<'p> {
    pieces: &'p [Piece],
}

struct Delimiter {
    formats: bool,
    holds_text: bool,
}

#[derive(Default)]
struct FormatScan {
    open: Vec<Delimiter>,
    in_text: bool,
    escaped: bool,
    written: String,
}

impl<'p> FormatArguments<'p> {
    const ARGUMENT_ENDS: [char; 2] = [',', ')'];

    pub fn new(pieces: &'p [Piece]) -> Self {
        Self { pieces }
    }

    pub fn holds_only(&self, argument: usize) -> bool {
        let mut uses = self
            .pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| Self::is_use_of(piece, argument))
            .peekable();
        uses.peek().is_some() && uses.all(|(position, _)| self.is_whole_argument(position))
    }

    fn is_use_of(piece: &Piece, argument: usize) -> bool {
        matches!(piece, Piece::Placeholder { index, .. } if *index == argument)
    }

    fn is_whole_argument(&self, position: usize) -> bool {
        self.ends_an_argument(position)
            && FormatScan::of(&self.pieces[..position]).starts_a_formatted_argument()
    }

    fn ends_an_argument(&self, position: usize) -> bool {
        matches!(
            self.pieces.get(position + 1),
            Some(Piece::Literal(text)) if text.trim_start().starts_with(Self::ARGUMENT_ENDS)
        )
    }
}

impl FormatScan {
    const MACROS: [&'static str; 18] = [
        "assert",
        "assert_eq",
        "assert_ne",
        "debug_assert",
        "debug_assert_eq",
        "debug_assert_ne",
        "eprint",
        "eprintln",
        "format",
        "format_args",
        "panic",
        "print",
        "println",
        "todo",
        "unimplemented",
        "unreachable",
        "write",
        "writeln",
    ];
    const PLACED_VALUE: char = '_';

    fn of(pieces: &[Piece]) -> Self {
        let mut scan = Self::default();
        for piece in pieces {
            match piece {
                Piece::Literal(text) => text.chars().for_each(|character| scan.read(character)),
                _ => scan.read(Self::PLACED_VALUE),
            }
        }
        scan
    }

    fn starts_a_formatted_argument(&self) -> bool {
        let follows_a_comma = self.written.trim_end().ends_with(',');
        follows_a_comma
            && !self.in_text
            && self
                .open
                .last()
                .is_some_and(|delimiter| delimiter.formats && delimiter.holds_text)
    }

    fn read(&mut self, character: char) {
        if self.in_text {
            self.read_text(character);
        } else {
            self.read_code(character);
        }
        self.written.push(character);
    }

    fn read_text(&mut self, character: char) {
        match (self.escaped, character) {
            (true, _) => self.escaped = false,
            (false, '\\') => self.escaped = true,
            (false, '"') => self.close_text(),
            _ => {}
        }
    }

    fn close_text(&mut self) {
        self.in_text = false;
        if let Some(delimiter) = self.open.last_mut() {
            delimiter.holds_text = true;
        }
    }

    fn read_code(&mut self, character: char) {
        match character {
            '"' => self.in_text = true,
            '(' => self.open.push(Delimiter {
                formats: self.calls_a_format_macro(),
                holds_text: false,
            }),
            '[' | '{' => self.open.push(Delimiter {
                formats: false,
                holds_text: false,
            }),
            ')' | ']' | '}' => {
                self.open.pop();
            }
            _ => {}
        }
    }

    fn calls_a_format_macro(&self) -> bool {
        let Some(before_bang) = self.written.trim_end().strip_suffix('!') else {
            return false;
        };
        let name_start = before_bang
            .rfind(|character: char| !(character.is_alphanumeric() || character == '_'))
            .map_or(0, |at| at + 1);
        Self::MACROS.contains(&&before_bang[name_start..])
    }
}

#[cfg(test)]
mod tests {
    use super::FormatArguments;
    use crate::core::ParsedTemplate;

    struct FormatFixture;

    impl FormatFixture {
        fn holds_only(text: &str, argument: usize) -> bool {
            let parsed = ParsedTemplate::parse(text).expect("a valid template");
            FormatArguments::new(parsed.pieces()).holds_only(argument)
        }
    }

    #[test]
    fn an_argument_after_the_format_string_is_formatted() {
        for text in [
            "println!(\"{{}}\", {0})",
            "format!(\"{{}} and {{}}\", {1}, {0})",
            "std::eprintln!(\"{{}}\", {0} )",
            "write!(out, \"{{}}\", {0})",
            "assert!(ok, \"{{}}\", {0})",
            "panic!(\"a (paren\\\" {{}}\", {0})",
            "println!(\"{{}}\", {&0})",
        ] {
            assert!(FormatFixture::holds_only(text, 0), "{text}");
        }
    }

    #[test]
    fn an_argument_before_the_format_string_or_inside_an_expression_is_not() {
        for text in [
            "write!({0}, \"{{}}\", 1)",
            "assert!({0}, \"{{}}\", 1)",
            "println!(\"{{}}\", {0}.len())",
            "println!(\"{{}}\", f({0}))",
            "vec!(\"a\", {0})",
            "println!(\"{{}}\", {0}); f({0})",
            "{0}",
        ] {
            assert!(!FormatFixture::holds_only(text, 0), "{text}");
        }
    }
}
