use super::operator::RustText;
use crate::core::Piece;

pub(super) struct FormatArguments<'p> {
    pieces: &'p [Piece],
}

impl<'p> FormatArguments<'p> {
    const MACROS: [(&'static str, usize); 18] = [
        ("assert", 1),
        ("assert_eq", 2),
        ("assert_ne", 2),
        ("debug_assert", 1),
        ("debug_assert_eq", 2),
        ("debug_assert_ne", 2),
        ("eprint", 0),
        ("eprintln", 0),
        ("format", 0),
        ("format_args", 0),
        ("panic", 0),
        ("print", 0),
        ("println", 0),
        ("todo", 0),
        ("unimplemented", 0),
        ("unreachable", 0),
        ("write", 1),
        ("writeln", 1),
    ];
    const PLACED_VALUE: &'static str = "_";

    pub(super) fn new(pieces: &'p [Piece]) -> Self {
        Self { pieces }
    }

    pub(super) fn formats_whole(&self, argument: usize) -> bool {
        let mut uses = self
            .pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| piece.is_placeholder_of(argument))
            .peekable();
        uses.peek().is_some() && uses.all(|(position, _)| self.is_whole_argument(position))
    }

    fn is_whole_argument(&self, position: usize) -> bool {
        let written = self.text_before(position);
        let before = RustText::new(&written);
        self.ends_an_argument(position)
            && before.starts_an_argument()
            && before
                .innermost_open()
                .is_some_and(|open| open.follows_format_string(&Self::MACROS))
    }

    fn ends_an_argument(&self, position: usize) -> bool {
        matches!(
            self.pieces.get(position + 1),
            Some(Piece::Literal(text)) if text.trim_start().starts_with(RustText::ARGUMENT_ENDS)
        )
    }

    fn text_before(&self, position: usize) -> String {
        self.pieces[..position]
            .iter()
            .map(|piece| match piece {
                Piece::Literal(text) => text.as_str(),
                _ => Self::PLACED_VALUE,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::FormatArguments;
    use crate::core::ParsedTemplate;

    struct FormatFixture;

    impl FormatFixture {
        fn formats_whole(text: &str, argument: usize) -> bool {
            let parsed = ParsedTemplate::parse(text).expect("a valid template");
            FormatArguments::new(parsed.pieces()).formats_whole(argument)
        }
    }

    #[test]
    fn an_argument_after_the_format_string_is_formatted() {
        for text in [
            "println!(\"{{}}\", {0})",
            "format!(\"{{}} and {{}}\", {1}, {0})",
            "std::eprintln!(\"{{}}\", {0} )",
            "write!(out, \"{{}}\", {0})",
            "write!(out, \"{{}}{{}}\", ')', {0})",
            "assert!(ok, \"{{}}\", {0})",
            "panic!(\"a (paren\\\" {{}}\", {0})",
            "println!(\"{{}}\", {&0})",
            "println!(\"{{}}\", [1][0], {0})",
            "println!(\"{{}}\", f(1), {0})",
            "println!(\"{{x}}\", x = {0})",
            "assert_eq!(\"a\", b, \"{{}}\", {0})",
            "write!(f(')'), \"{{}}\", {0})",
        ] {
            assert!(FormatFixture::formats_whole(text, 0), "{text}");
        }
    }

    #[test]
    fn an_argument_before_the_format_string_or_inside_an_expression_is_not() {
        for text in [
            "write!({0}, \"{{}}\", 1)",
            "assert!({0}, \"{{}}\", 1)",
            "println!(\"{{}}\", {0}.len())",
            "println!(\"{{}}\", f({0}))",
            "println!(\"{{}}\", [{0}])",
            "format!(\"{{}}\", vec![\"a\", {0}, 1])",
            "f(\"a\", {0})",
            "println!(\"{{}}\", {1}{0})",
            "assert_eq!(\"a\", {0})",
            "assert_ne!(\"x\", {0}, \"{{}}\", 1)",
            "assert!(a == \"x\", {0})",
            "println!(\"{{}}\", x == {0})",
            "format!(\"{{}}\", g(')', \"x\", {0}))",
            "vec!(\"a\", {0})",
            "println!(\"{{}}\", {0}); f({0})",
            "{0}",
        ] {
            assert!(!FormatFixture::formats_whole(text, 0), "{text}");
        }
    }

    #[test]
    fn an_argument_the_template_never_writes_is_not_formatted() {
        assert!(!FormatFixture::formats_whole("println!(\"{{}}\", {0})", 1));
    }
}
