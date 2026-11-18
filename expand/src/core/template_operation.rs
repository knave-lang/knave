use super::PrimitiveType;
use super::lexeme::Lexeme;
use super::template::Piece;
use super::template_lexer::TemplateLexer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    Argument(usize),
    Constant(Option<i128>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Division {
    pub dividend: Option<Operand>,
    pub divisor: Operand,
}

pub struct TemplateOperation<'t> {
    lexemes: Vec<Lexeme<'t>>,
    quotes_an_argument: bool,
}

impl<'t> TemplateOperation<'t> {
    const OPERATORS: &'static str = "+-*/%!<>=&|^";
    const DIVISIONS: &'static str = "/%";
    const CAST: &'static str = "as";
    const NEGATION: char = '-';
    const CALL: char = '(';
    const MEMBER: char = '.';
    const MACRO: char = '!';
    const ASSIGNMENT: char = '=';
    const COMPARISON_STARTS: &'static str = "!<>";
    const PURE_SYMBOLS: &'static str = "+-*/%<>^.:()[],";
    const IMPURE_WORDS: [&'static str; 17] = [
        "async", "break", "continue", "else", "for", "if", "let", "loop", "match", "move", "mut",
        "ref", "return", "static", "unsafe", "while", "yield",
    ];
    const GENERICS_START: char = '<';
    const GENERICS_END: char = '>';
    const PATH_SEPARATOR: char = ':';
    const OPENING_BRACKETS: &'static str = "([{";
    const CLOSING_BRACKETS: &'static str = ")]}";
    const PROMOTABLE_FUNCTIONS: [&'static str; 6] = [
        "align_of",
        "max_value",
        "min_value",
        "null",
        "null_mut",
        "size_of",
    ];
    const LITERAL_MACROS: [&'static str; 9] = [
        "cfg",
        "column",
        "concat",
        "env",
        "file",
        "line",
        "module_path",
        "option_env",
        "stringify",
    ];

    pub fn of(pieces: &'t [Piece]) -> Self {
        let mut lexer = TemplateLexer::default();
        let mut quotes_an_argument = false;
        let lexemes = pieces
            .iter()
            .flat_map(|piece| match piece {
                Piece::Literal(literal) => lexer.lex(literal),
                Piece::Placeholder { index, .. } if lexer.in_code() => {
                    vec![Lexeme::Argument(*index)]
                }
                Piece::Placeholder { .. } => {
                    quotes_an_argument = true;
                    Vec::new()
                }
                Piece::TypePlaceholder { .. } | Piece::BinderPlaceholder { .. } => Vec::new(),
            })
            .collect();
        Self {
            lexemes: Self::without_literal_macro_arguments(lexemes),
            quotes_an_argument,
        }
    }

    pub fn calls(&self) -> bool {
        self.calls_function() || self.calls_macro()
    }

    pub fn operator_arguments(&self) -> impl Iterator<Item = usize> + '_ {
        self.lexemes.windows(2).filter_map(|pair| match pair {
            [Lexeme::Argument(index), Lexeme::Symbol(symbol)]
            | [Lexeme::Symbol(symbol), Lexeme::Argument(index)]
                if Self::OPERATORS.contains(*symbol) =>
            {
                Some(*index)
            }
            _ => None,
        })
    }

    pub fn is_pure(&self) -> bool {
        !self.quotes_an_argument
            && (0..self.lexemes.len()).all(|position| self.is_pure_at(position))
    }

    pub fn is_logical_not(&self) -> bool {
        matches!(
            self.lexemes.as_slice(),
            [Lexeme::Symbol(Self::MACRO), Lexeme::Argument(_)]
        )
    }

    pub fn is_negation(&self) -> bool {
        matches!(
            self.lexemes.as_slice(),
            [Lexeme::Symbol(Self::NEGATION), Lexeme::Argument(_)]
        )
    }

    pub fn divisions(&self) -> Vec<Division> {
        self.lexemes
            .iter()
            .enumerate()
            .filter(|(_, lexeme)| Self::divides(lexeme))
            .filter_map(|(position, _)| self.division_at(position))
            .collect()
    }

    fn is_pure_at(&self, position: usize) -> bool {
        match self.lexemes[position] {
            Lexeme::Word(word) => !Self::IMPURE_WORDS.contains(&word),
            Lexeme::Symbol(symbol) => self.is_pure_symbol(position, symbol),
            Lexeme::Argument(_) => !self.calls_through_argument_at(position),
            Lexeme::Number(_) | Lexeme::Text => true,
        }
    }

    fn is_pure_symbol(&self, position: usize, symbol: char) -> bool {
        match symbol {
            Self::MACRO => !self.calls_macro_at(position),
            Self::ASSIGNMENT => self.compares_at(position),
            Self::MEMBER => !self.calls_method_at(position),
            other => Self::PURE_SYMBOLS.contains(other),
        }
    }

    fn calls_through_argument_at(&self, argument: usize) -> bool {
        match self.lexemes.get(argument + 1..).unwrap_or_default() {
            [Lexeme::Symbol(Self::CALL), ..] => true,
            [
                Lexeme::Symbol(Self::MACRO),
                Lexeme::Symbol(Self::ASSIGNMENT),
                ..,
            ] => false,
            [Lexeme::Symbol(Self::MACRO), ..] => true,
            _ => false,
        }
    }

    fn calls_method_at(&self, member: usize) -> bool {
        matches!(
            self.lexemes.get(member + 1..).unwrap_or_default(),
            [Lexeme::Argument(_), ..]
                | [
                    Lexeme::Word(_),
                    Lexeme::Symbol(Self::CALL | Self::PATH_SEPARATOR),
                    ..
                ]
        )
    }

    fn calls_macro_at(&self, position: usize) -> bool {
        let after_a_word = position
            .checked_sub(1)
            .is_some_and(|before| matches!(self.lexemes[before], Lexeme::Word(_)));
        after_a_word && self.lexemes.get(position + 1) != Some(&Lexeme::Symbol(Self::ASSIGNMENT))
    }

    fn compares_at(&self, position: usize) -> bool {
        let symbol_at = |offset: Option<usize>| match offset.and_then(|at| self.lexemes.get(at)) {
            Some(Lexeme::Symbol(symbol)) => Some(*symbol),
            _ => None,
        };
        let before = symbol_at(position.checked_sub(1));
        let earlier = symbol_at(position.checked_sub(2));
        match (before, symbol_at(Some(position + 1))) {
            (_, Some(Self::ASSIGNMENT)) | (Some(Self::ASSIGNMENT), _) => true,
            (Some(start), _) => Self::COMPARISON_STARTS.contains(start) && earlier != Some(start),
            (None, _) => false,
        }
    }

    fn without_literal_macro_arguments(mut lexemes: Vec<Lexeme<'t>>) -> Vec<Lexeme<'t>> {
        let mut position = 0;
        while position < lexemes.len() {
            if Self::opens_literal_macro(&lexemes[position..]) {
                let open = position + 2;
                let close = Self::matching_close(&lexemes, open);
                lexemes.drain(open + 1..close);
            }
            position += 1;
        }
        lexemes
    }

    fn opens_literal_macro(lexemes: &[Lexeme]) -> bool {
        matches!(
            lexemes,
            [Lexeme::Word(name), Lexeme::Symbol(Self::MACRO), Lexeme::Symbol(open), ..]
                if Self::LITERAL_MACROS.contains(name) && Self::OPENING_BRACKETS.contains(*open)
        )
    }

    fn matching_close(lexemes: &[Lexeme], open: usize) -> usize {
        let mut depth = 0;
        (open..lexemes.len())
            .find(|&position| {
                depth += Self::bracket_depth(&lexemes[position]);
                depth == 0
            })
            .unwrap_or(lexemes.len())
    }

    fn bracket_depth(lexeme: &Lexeme) -> i32 {
        match lexeme {
            Lexeme::Symbol(symbol) if Self::OPENING_BRACKETS.contains(*symbol) => 1,
            Lexeme::Symbol(symbol) if Self::CLOSING_BRACKETS.contains(*symbol) => -1,
            _ => 0,
        }
    }

    fn calls_function(&self) -> bool {
        self.positions_of(Self::CALL)
            .filter_map(|open| self.callee(open))
            .any(Self::names_function)
    }

    fn calls_macro(&self) -> bool {
        self.positions_of(Self::MACRO).any(|position| {
            self.calls_macro_at(position)
                && !matches!(self.lexemes[position - 1],
                    Lexeme::Word(name) if Self::LITERAL_MACROS.contains(&name))
        })
    }

    fn positions_of(&self, symbol: char) -> impl Iterator<Item = usize> + '_ {
        (0..self.lexemes.len())
            .filter(move |&position| self.lexemes[position] == Lexeme::Symbol(symbol))
    }

    fn callee(&self, open: usize) -> Option<&'t str> {
        let before = open.checked_sub(1)?;
        match self.lexemes[before] {
            Lexeme::Word(name) if name != Self::CAST => Some(name),
            Lexeme::Symbol(Self::GENERICS_END) => self.turbofish_callee(before),
            _ => None,
        }
    }

    fn turbofish_callee(&self, end: usize) -> Option<&'t str> {
        let start = self.generics_start(end)?;
        match self.lexemes.get(start.checked_sub(3)?..start)? {
            [
                Lexeme::Word(name),
                Lexeme::Symbol(Self::PATH_SEPARATOR),
                Lexeme::Symbol(Self::PATH_SEPARATOR),
            ] => Some(name),
            _ => None,
        }
    }

    fn generics_start(&self, end: usize) -> Option<usize> {
        let mut depth = 0;
        (0..=end).rev().find(|&position| {
            depth += self.nesting(position);
            depth == 0
        })
    }

    fn nesting(&self, position: usize) -> i32 {
        match self.lexemes[position] {
            Lexeme::Symbol(Self::GENERICS_END) => 1,
            Lexeme::Symbol(Self::GENERICS_START) => -1,
            _ => 0,
        }
    }

    fn names_function(name: &str) -> bool {
        !Self::names_constructor(name) && !Self::PROMOTABLE_FUNCTIONS.contains(&name)
    }

    fn names_constructor(name: &str) -> bool {
        name.starts_with(char::is_uppercase)
    }

    fn divides(lexeme: &Lexeme) -> bool {
        matches!(lexeme, Lexeme::Symbol(symbol) if Self::DIVISIONS.contains(*symbol))
    }

    fn division_at(&self, position: usize) -> Option<Division> {
        let divisor = self.operand_after(position + 1)?;
        let dividend = position
            .checked_sub(1)
            .and_then(|before| self.operand(before));
        Some(Division { dividend, divisor })
    }

    fn operand_after(&self, position: usize) -> Option<Operand> {
        match self.lexemes.get(position)? {
            Lexeme::Symbol(Self::NEGATION) => self.negated_operand(position + 1),
            _ => self.operand(position),
        }
    }

    fn negated_operand(&self, position: usize) -> Option<Operand> {
        match self.lexemes.get(position)? {
            Lexeme::Number(text) => Some(Operand::Constant(
                Self::integer_value(text).map(|value| -value),
            )),
            _ => None,
        }
    }

    fn operand(&self, position: usize) -> Option<Operand> {
        match self.lexemes.get(position)? {
            Lexeme::Argument(index) => Some(Operand::Argument(*index)),
            Lexeme::Number(text) => Some(Operand::Constant(Self::integer_value(text))),
            Lexeme::Word(_) | Lexeme::Text | Lexeme::Symbol(_) => None,
        }
    }

    fn integer_value(text: &str) -> Option<i128> {
        let digits = PrimitiveType::all()
            .filter(|primitive| primitive.is_integer())
            .find_map(|primitive| text.strip_suffix(primitive.names().rust()))
            .unwrap_or(text);
        TemplateLexer::digits(digits).parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{Division, Operand};
    use crate::core::ParsedTemplate;

    struct OperationFixture;

    impl OperationFixture {
        fn parsed(text: &str) -> ParsedTemplate {
            ParsedTemplate::parse(text).expect("a valid template")
        }

        fn calls(text: &str) -> bool {
            Self::parsed(text).operation().calls()
        }

        fn operator_arguments(text: &str) -> Vec<usize> {
            Self::parsed(text)
                .operation()
                .operator_arguments()
                .collect()
        }

        fn is_pure(text: &str) -> bool {
            Self::parsed(text).operation().is_pure()
        }

        fn divisions(text: &str) -> Vec<Division> {
            Self::parsed(text).operation().divisions()
        }
    }

    #[test]
    fn finds_no_call_in_an_operator() {
        for text in [
            "{0} + {1}",
            "-{0}",
            "!{0}",
            "{&0} <= {&1}",
            "{0} + 1",
            "{0} * 2_i64",
            "{0} * 1.5",
            "{0} * 2.",
            "{0} as i64",
            "{0} as (i64)",
            "{0} as f64 / 2.0",
            "{0} == true",
            "{0} == 'a'",
            "i64::MAX - {0}",
            "true != {0}",
            "{0}..10",
            "&{0}[{1}]",
            "{0}.line != 1",
            "[true, {0} < 2, 3 > (1)]",
        ] {
            assert!(!OperationFixture::calls(text), "{text}");
        }
    }

    #[test]
    fn finds_no_call_in_a_value_that_rust_promotes() {
        for text in [
            "({0}, 1)",
            "()",
            "Some({0})",
            "None::<i64>",
            "(\"f(\", {0})",
            "(\"a\\\"vec![\", {0})",
            "std::mem::size_of::<i64>() + {0}",
            "stringify!({0})",
            "|{!item}| {0}",
            "cfg!(any(unix, windows)) == {0}",
            "(stringify!(f({0})), 1)",
            "(stringify!(vec![{0}]), 1)",
        ] {
            assert!(!OperationFixture::calls(text), "{text}");
        }
    }

    #[test]
    fn finds_a_function_or_a_method_call() {
        for text in [
            "{0}.len()",
            "Box::new({0})",
            "{0}.parse::<{target}>()",
            "{0}.collect::<Vec<_>>()",
            "{0}.max(1)",
            "{0} + 1.max(2)",
            "{0} + 1 .max(2)",
            "(\"Leaf\", 1.max({0}))",
            "(stringify!(f({0})), {0}.len())",
        ] {
            assert!(OperationFixture::calls(text), "{text}");
        }
    }

    #[test]
    fn finds_a_macro_call() {
        for text in ["vec![{0}]", "dbg!({0}) + 1"] {
            assert!(OperationFixture::calls(text), "{text}");
        }
    }

    #[test]
    fn names_the_arguments_an_operator_touches() {
        assert_eq!(OperationFixture::operator_arguments("{0} == {1}"), [0, 1]);
        assert_eq!(OperationFixture::operator_arguments("-{0}"), [0]);
        assert_eq!(OperationFixture::operator_arguments("({0}, {1} + 1)"), [1]);
        assert!(OperationFixture::operator_arguments("({0}, -1)").is_empty());
        assert!(OperationFixture::operator_arguments("{0} as i64").is_empty());
        assert_eq!(
            OperationFixture::operator_arguments("{0} /* {1} */ % 2"),
            [0]
        );
        assert!(OperationFixture::operator_arguments("\"{0}\" == \"a\"").is_empty());
    }

    #[test]
    fn names_a_negation() {
        assert!(OperationFixture::parsed("-{0}").operation().is_negation());
        assert!(
            !OperationFixture::parsed("{0} - {1}")
                .operation()
                .is_negation()
        );
        assert!(!OperationFixture::parsed("!{0}").operation().is_negation());
    }

    #[test]
    fn finds_a_division_between_placeholders() {
        assert_eq!(
            OperationFixture::divisions("{1} % {0}"),
            [Division {
                dividend: Some(Operand::Argument(1)),
                divisor: Operand::Argument(0),
            }]
        );
        assert!(OperationFixture::divisions("{0} + {1}").is_empty());
    }

    #[test]
    fn reads_a_number_divisor() {
        for (text, value) in [
            ("{0} / 0", Some(0)),
            ("{0} % -1", Some(-1)),
            ("{0} / 1_000_i64", Some(1000)),
            ("{0} / 2.5", None),
            ("{0} / 0.", None),
        ] {
            assert_eq!(
                OperationFixture::divisions(text),
                [Division {
                    dividend: Some(Operand::Argument(0)),
                    divisor: Operand::Constant(value),
                }],
                "{text}"
            );
        }
    }

    #[test]
    fn reads_a_number_dividend() {
        assert_eq!(
            OperationFixture::divisions("10 / {0}"),
            [Division {
                dividend: Some(Operand::Constant(Some(10))),
                divisor: Operand::Argument(0),
            }]
        );
        assert_eq!(
            OperationFixture::divisions("({0}) / {1}"),
            [Division {
                dividend: None,
                divisor: Operand::Argument(1),
            }]
        );
    }

    #[test]
    fn reads_a_division_only_where_it_runs() {
        assert!(OperationFixture::divisions("stringify!({0} / 0)").is_empty());
        assert_eq!(
            OperationFixture::divisions("vec![{0} / 0]"),
            [Division {
                dividend: Some(Operand::Argument(0)),
                divisor: Operand::Constant(Some(0)),
            }]
        );
    }

    #[test]
    fn skips_a_division_by_a_negated_placeholder() {
        assert!(OperationFixture::divisions("{0} / -{1}").is_empty());
        assert!(OperationFixture::divisions("{0} / ({1})").is_empty());
    }

    #[test]
    fn finds_a_plain_expression_pure() {
        for text in [
            "{0} % 2 == 0",
            "{0} % 2 != 0",
            "{0} <= 1",
            "{0} >= 1",
            "u8::is_multiple_of({0}, 2)",
            "!u8::is_multiple_of({0}, 2)",
            "[{0}, 1][0]",
            "f::<i64>({0}).0 + str::len(\"a=\")",
            "{0} + 1 != i64::MAX",
            "{0} != 0",
            "({0}, 1).1",
            "{0} + 1 // a = b",
        ] {
            assert!(OperationFixture::is_pure(text), "{text}");
        }
    }

    #[test]
    fn finds_any_text_that_could_write_impure() {
        for text in [
            "{0} = 7",
            "({0}) += 1",
            "x <<= {0}",
            "x >>= {0}",
            "&{0}",
            "{0} && true",
            "{{ ({0}, _) = (7, 0); true }}",
            "{{ [{0}, _] = [7, 0]; true }}",
            "{{ ({0},) = (7,); true }}",
            "{{ let p = &raw mut {0}; unsafe {{ *p += 1 }}; true }}",
            "{{ *{1} += 1; match {0} {{ ref mut r => *r += 1 }}; true }}",
            "println!(\"{{}}\", {0})",
            "std::ptr::addr_of_mut!({0})",
            "(|x: i64| x + 1)({0})",
            "{0}?",
            "() == {0}.make_ascii_uppercase()",
            "[{0}.{1}()][0]",
            "{0}.{1} + 1",
            "{1}({0})",
            "{1}!({0})",
            "/* {1} */ {0} + 1",
            "\"{1}\" != \"\"",
            "{0}.max(1) as u32",
            "{0}.wrapping_mul::<i64>(2)",
            "return {0} % 2 == 0",
            "{0} | 1 == 1",
            "if {0} {{ 1 }} else {{ 2 }}",
        ] {
            assert!(!OperationFixture::is_pure(text), "{text}");
        }
    }
}
