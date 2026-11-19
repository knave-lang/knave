use super::operator::{OpenDelimiter, RustText};
use crate::core::Piece;

pub(super) struct FormatArguments<'p> {
    pieces: &'p [Piece],
}

struct FormatMacro {
    name: &'static str,
    format_string: usize,
}

struct FormatString<'t> {
    text: &'t str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatArgument<'t> {
    Positional(usize),
    Named(&'t str),
}

impl<'p> FormatArguments<'p> {
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
        let Some(start) = RustText::new(&written).argument_start() else {
            return false;
        };
        let Some(format_macro) = FormatMacro::called_by(start.open()) else {
            return false;
        };
        let argument = match start.name() {
            Some(name) => FormatArgument::Named(name),
            None => FormatArgument::Positional(format_macro.positional_index(start.open())),
        };
        self.ends_an_argument(position)
            && format_macro
                .format_string(start.open())
                .is_some_and(|format_string| !format_string.points_at(argument))
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

impl FormatMacro {
    const ALL: [Self; 18] = [
        Self::at("assert", 1),
        Self::at("assert_eq", 2),
        Self::at("assert_ne", 2),
        Self::at("debug_assert", 1),
        Self::at("debug_assert_eq", 2),
        Self::at("debug_assert_ne", 2),
        Self::at("eprint", 0),
        Self::at("eprintln", 0),
        Self::at("format", 0),
        Self::at("format_args", 0),
        Self::at("panic", 0),
        Self::at("print", 0),
        Self::at("println", 0),
        Self::at("todo", 0),
        Self::at("unimplemented", 0),
        Self::at("unreachable", 0),
        Self::at("write", 1),
        Self::at("writeln", 1),
    ];

    const fn at(name: &'static str, format_string: usize) -> Self {
        Self {
            name,
            format_string,
        }
    }

    fn called_by(open: &OpenDelimiter<'_>) -> Option<&'static Self> {
        Self::ALL
            .iter()
            .find(|format_macro| open.callee() == Some(format_macro.name))
    }

    fn format_string<'t>(&self, open: &OpenDelimiter<'t>) -> Option<FormatString<'t>> {
        if open.passed_arguments() <= self.format_string {
            return None;
        }
        open.text_at(self.format_string)
            .map(|text| FormatString { text })
    }

    fn positional_index(&self, open: &OpenDelimiter<'_>) -> usize {
        open.passed_arguments()
            .saturating_sub(self.format_string + 1)
    }
}

impl<'t> FormatString<'t> {
    const POINTER: char = 'p';
    const PRECISION_FROM_ARGUMENT: &'static str = ".*";

    fn points_at(&self, argument: FormatArgument<'_>) -> bool {
        let mut next_positional = 0;
        self.placeholders().any(|placeholder| {
            let (written, spec) = placeholder.split_once(':').unwrap_or((placeholder, ""));
            let written = written.trim();
            if written.is_empty() && spec.contains(Self::PRECISION_FROM_ARGUMENT) {
                next_positional += 1;
            }
            let formatted = match written.parse() {
                Ok(index) => FormatArgument::Positional(index),
                Err(_) if written.is_empty() => {
                    next_positional += 1;
                    FormatArgument::Positional(next_positional - 1)
                }
                Err(_) => FormatArgument::Named(written),
            };
            formatted == argument && spec.trim_end().ends_with(Self::POINTER)
        })
    }

    fn placeholders(&self) -> impl Iterator<Item = &'t str> {
        self.text
            .split("{{")
            .flat_map(|part| part.split('{').skip(1))
            .filter_map(|opened| opened.split_once('}').map(|(inside, _)| inside))
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
            "println!(\"{{x}}\",x = {0})",
            "println!(\"{{:p}} {{}}\", 1, {0})",
            "println!(\"{{:.*}}\", 2, {0})",
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
            "assert_eq!(a, b, 'c', {0})",
            "println!(\"{{:p}}\", {0})",
            "println!(\"{{}} {{:#p}}\", 1, {0})",
            "println!(\"{{0:p}}\", {0})",
            "println!(\"{{x:p}}\", x = {0})",
            "println!(\"{{:.*}} {{:p}}\", 2, 1.5, {0})",
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
