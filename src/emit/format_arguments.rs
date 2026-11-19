use super::open_delimiter::OpenDelimiter;
use super::operator::{ArgumentStart, RustText};
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

struct FormatPlaceholder<'t> {
    argument: FormatArgument<'t>,
    spec: &'t str,
}

struct FormatPlaceholders<'t> {
    written: std::vec::IntoIter<&'t str>,
    next_positional: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatArgument<'t> {
    Positional(usize),
    Named(&'t str),
}

struct PassedArgument<'t> {
    position: usize,
    name: Option<&'t str>,
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
        self.ends_an_argument(position)
            && format_macro
                .format_string(start.open())
                .is_some_and(|format_string| {
                    !format_string.points_at(&format_macro.argument_at(&start))
                })
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

    fn argument_at<'t>(&self, start: &ArgumentStart<'t>) -> PassedArgument<'t> {
        let passed = start.open().passed_arguments();
        PassedArgument {
            position: passed.saturating_sub(self.format_string + 1),
            name: start.name(),
        }
    }
}

impl<'t> FormatString<'t> {
    fn points_at(&self, argument: &PassedArgument<'_>) -> bool {
        FormatPlaceholders::of(self.text)
            .any(|placeholder| argument.is(placeholder.argument) && placeholder.is_pointer())
    }
}

impl PassedArgument<'_> {
    fn is(&self, formatted: FormatArgument<'_>) -> bool {
        match formatted {
            FormatArgument::Positional(position) => position == self.position,
            FormatArgument::Named(name) => Some(name) == self.name,
        }
    }
}

impl FormatPlaceholder<'_> {
    const POINTER: char = 'p';

    fn is_pointer(&self) -> bool {
        self.spec.trim_end().ends_with(Self::POINTER)
    }
}

impl<'t> FormatPlaceholders<'t> {
    const PRECISION_FROM_ARGUMENT: &'static str = ".*";

    fn of(text: &'t str) -> Self {
        let written: Vec<&'t str> = text
            .split("{{")
            .flat_map(|part| part.split('{').skip(1))
            .filter_map(|opened| opened.split_once('}').map(|(inside, _)| inside))
            .collect();
        Self {
            written: written.into_iter(),
            next_positional: 0,
        }
    }

    fn resolve(&mut self, written: &'t str, spec: &str) -> FormatArgument<'t> {
        if spec.contains(Self::PRECISION_FROM_ARGUMENT) {
            self.next_positional += 1;
        }
        match written.parse() {
            Ok(index) => FormatArgument::Positional(index),
            Err(_) if written.is_empty() => self.take_positional(),
            Err(_) => FormatArgument::Named(written),
        }
    }

    fn take_positional(&mut self) -> FormatArgument<'t> {
        let argument = FormatArgument::Positional(self.next_positional);
        self.next_positional += 1;
        argument
    }
}

impl<'t> Iterator for FormatPlaceholders<'t> {
    type Item = FormatPlaceholder<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        let placeholder = self.written.next()?;
        let (written, spec) = placeholder.split_once(':').unwrap_or((placeholder, ""));
        let argument = self.resolve(written.trim(), spec);
        Some(FormatPlaceholder { argument, spec })
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
            "println!(\"{{{{:p}}}} {{}}\", {0})",
            "println!(\"}}}}{{:p}} {{}}\", 1, {0})",
            "{{ f(\"{{}}\", 1); println!(\"{{:p}} {{}}\", 1, {0}) }}",
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
            "println!(\"{{x:.*}} {{:p}}\", 2, {0}, x = 1.5)",
            "println!(\"{{2:.*}} {{:p}}\", 3, {0}, 1.5)",
            "println!(\"{{:p }}\", {0})",
            "println!(\"{{0 :p}}\", {0})",
            "println!(\"{{ :p}}\", {0})",
            "println!(\"{{:p}}\", x = {0})",
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
