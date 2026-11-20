use std::collections::BTreeMap;

use super::argument_start::ArgumentStart;
use super::format_string::{FormatString, PassedArgument};
use super::macro_call::MacroCall;
use super::open_delimiter::OpenDelimiter;
use super::operator::RustText;
use crate::core::Piece;

pub(super) struct FormatArguments<'p> {
    pieces: &'p [Piece],
}

struct PlacedText {
    written: String,
    uses: Vec<PlacedArgument>,
}

struct PlacedArgument {
    argument: usize,
    position: usize,
    offset: usize,
}

struct JudgedUse {
    argument: usize,
    whole: bool,
}

struct FormatMacro {
    name: &'static str,
    format_string: usize,
}

impl<'p> FormatArguments<'p> {
    const PLACED_VALUE: &'static str = "_";

    pub(super) fn new(pieces: &'p [Piece]) -> Self {
        Self { pieces }
    }

    pub(super) fn formatted(&self) -> Vec<usize> {
        if !self.calls_a_format_macro() {
            return Vec::new();
        }
        let placed = self.placed_text();
        let starts = RustText::new(&placed.written).argument_starts(&placed.offsets());
        let judged = placed
            .uses
            .iter()
            .zip(starts)
            .map(|(placed, start)| JudgedUse {
                argument: placed.argument,
                whole: self.is_whole_argument(placed.position, start.as_ref()),
            });
        Self::arguments_whole_at_every_use(judged)
    }

    fn arguments_whole_at_every_use(judged: impl Iterator<Item = JudgedUse>) -> Vec<usize> {
        let mut every_use_whole = BTreeMap::new();
        for judged_use in judged {
            every_use_whole
                .entry(judged_use.argument)
                .and_modify(|all: &mut bool| *all &= judged_use.whole)
                .or_insert(judged_use.whole);
        }
        every_use_whole
            .into_iter()
            .filter_map(|(argument, all)| all.then_some(argument))
            .collect()
    }

    fn calls_a_format_macro(&self) -> bool {
        self.pieces.iter().any(|piece| match piece {
            Piece::Literal(text) => FormatMacro::is_called_in(text),
            _ => false,
        })
    }

    fn is_whole_argument(&self, position: usize, start: Option<&ArgumentStart<'_>>) -> bool {
        let Some(start) = start else {
            return false;
        };
        let Some(format_macro) = FormatMacro::called_by(start.open()) else {
            return false;
        };
        self.ends_an_argument(position)
            && format_macro
                .format_string(start.open())
                .is_some_and(|format_string| {
                    !format_string.points_at(&format_macro.argument_at(start))
                })
    }

    fn ends_an_argument(&self, position: usize) -> bool {
        matches!(
            self.pieces.get(position + 1),
            Some(Piece::Literal(text)) if text.trim_start().starts_with(RustText::ARGUMENT_ENDS)
        )
    }

    fn placed_text(&self) -> PlacedText {
        let mut written = String::new();
        let mut uses = Vec::new();
        for (position, piece) in self.pieces.iter().enumerate() {
            match piece {
                Piece::Literal(text) => written.push_str(text),
                Piece::Placeholder { index, .. } => {
                    uses.push(PlacedArgument {
                        argument: *index,
                        position,
                        offset: written.len(),
                    });
                    written.push_str(Self::PLACED_VALUE);
                }
                _ => written.push_str(Self::PLACED_VALUE),
            }
        }
        PlacedText { written, uses }
    }
}

impl PlacedText {
    fn offsets(&self) -> Vec<usize> {
        self.uses.iter().map(|placed| placed.offset).collect()
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

    const CALL_OPENS: char = '(';

    fn is_called_in(text: &str) -> bool {
        text.match_indices(Self::CALL_OPENS)
            .any(|(parenthesis, _)| {
                MacroCall::ending(&text[..parenthesis])
                    .is_some_and(|call| Self::named(call.name()).is_some())
            })
    }

    fn called_by(open: &OpenDelimiter<'_>) -> Option<&'static Self> {
        Self::named(open.callee()?)
    }

    fn named(name: &str) -> Option<&'static Self> {
        Self::ALL
            .iter()
            .find(|format_macro| format_macro.name == name)
    }

    fn format_string<'t>(&self, open: &OpenDelimiter<'t>) -> Option<FormatString<'t>> {
        if open.passed_arguments() <= self.format_string {
            return None;
        }
        open.text_at(self.format_string).map(FormatString::new)
    }

    fn argument_at<'t>(&self, start: &ArgumentStart<'t>) -> PassedArgument<'t> {
        let passed = start.open().passed_arguments();
        PassedArgument::new(passed.saturating_sub(self.format_string + 1), start.name())
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
            FormatArguments::new(parsed.pieces())
                .formatted()
                .contains(&argument)
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
            "println! (\"{{}}\", {0})",
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
            "println!(\"{{:.*p}}\", 2, {0})",
            "println!(\"{{x}} {{:p}}\", {0}, x = 1)",
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
