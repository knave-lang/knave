use super::callee_definitions::{CalleeDefinition, CalleeDefinitions};
use crate::core::{ParsedTemplate, Piece, RustTemplate};
use crate::types::{TypedArgument, TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr};

#[derive(Clone, Copy)]
pub struct ShortForm<'p> {
    callees: CalleeDefinitions<'p>,
}

impl<'p> ShortForm<'p> {
    pub fn new(callees: CalleeDefinitions<'p>) -> Self {
        Self { callees }
    }

    pub fn admits(&self, path: &[&TypedExpr]) -> bool {
        path.windows(2)
            .all(|step| self.admits_step(step[0], step[1]))
    }

    fn admits_step(&self, parent: &TypedExpr, child: &TypedExpr) -> bool {
        match &parent.kind {
            TypedExprKind::Call(callee, arguments) => {
                let lent = arguments
                    .iter()
                    .take_while(|argument| !std::ptr::eq(&argument.value, child))
                    .count();
                arguments[..lent]
                    .iter()
                    .all(|argument| Self::is_literal(&argument.value))
                    && self.admits_callee(callee, arguments, lent)
            }
            TypedExprKind::Struct(TypedStructExpr::New(_, fields)) => {
                Self::only_literals_ahead(fields.iter().map(|(_, value)| value), child)
            }
            TypedExprKind::Variant(_, values, _) => Self::only_literals_ahead(values.iter(), child),
            TypedExprKind::Borrow(..)
            | TypedExprKind::Struct(TypedStructExpr::Field(..))
            | TypedExprKind::Try { .. } => true,
            _ => false,
        }
    }

    fn only_literals_ahead<'e>(
        operands: impl Iterator<Item = &'e TypedExpr>,
        child: &TypedExpr,
    ) -> bool {
        operands
            .take_while(|operand| !std::ptr::eq(*operand, child))
            .all(Self::is_literal)
    }

    fn is_literal(operand: &TypedExpr) -> bool {
        matches!(operand.kind, TypedExprKind::Leaf(TypedLeaf::Literal(_)))
    }

    fn admits_callee(&self, callee: &TypedExpr, arguments: &[TypedArgument], lent: usize) -> bool {
        match self.callees.of(callee) {
            CalleeDefinition::Function => true,
            CalleeDefinition::Template { template, .. } => {
                TemplateShape::new(template, arguments, lent).runs_nothing_ahead()
            }
            CalleeDefinition::Unknown => false,
        }
    }
}

struct TemplateShape<'t> {
    template: &'t RustTemplate,
    arguments: &'t [TypedArgument],
    lent: usize,
}

impl<'t> TemplateShape<'t> {
    const FORBIDDEN_CHARACTERS: [char; 3] = ['{', '|', ';'];
    const FORBIDDEN_OPERATORS: [&'static str; 2] = ["&&", "?"];

    fn new(template: &'t RustTemplate, arguments: &'t [TypedArgument], lent: usize) -> Self {
        Self {
            template,
            arguments,
            lent,
        }
    }

    fn runs_nothing_ahead(&self) -> bool {
        std::iter::once(&self.template.template)
            .chain(self.template.clauses.iter().map(|clause| &clause.template))
            .all(|text| {
                self.admits(
                    &ParsedTemplate::parse(text).expect("the expansion checked the template"),
                )
            })
    }

    fn admits(&self, parsed: &ParsedTemplate) -> bool {
        Self::is_straight_line(parsed)
            && Self::uses_each_placeholder_once(parsed)
            && self.runs_only_literals_before_the_lent_placeholder(parsed)
    }

    fn is_straight_line(parsed: &ParsedTemplate) -> bool {
        Self::literals(parsed).all(|literal| {
            !literal.contains(Self::FORBIDDEN_CHARACTERS)
                && !Self::FORBIDDEN_OPERATORS
                    .iter()
                    .any(|operator| literal.contains(operator))
        })
    }

    fn uses_each_placeholder_once(parsed: &ParsedTemplate) -> bool {
        let mut seen: Vec<usize> = parsed.placeholders().collect();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        seen.len() == count
    }

    fn runs_only_literals_before_the_lent_placeholder(&self, parsed: &ParsedTemplate) -> bool {
        let mut prefix = PathPrefix::default();
        parsed
            .pieces()
            .iter()
            .take_while(
                |piece| !matches!(piece, Piece::Placeholder { index, .. } if *index == self.lent),
            )
            .all(|piece| match piece {
                Piece::Literal(literal) => prefix.extend_with(literal),
                Piece::Placeholder { index, .. } => self
                    .arguments
                    .get(*index)
                    .is_some_and(|argument| ShortForm::is_literal(&argument.value)),
                Piece::TypePlaceholder { .. } | Piece::BinderPlaceholder { .. } => true,
            })
    }

    fn literals(parsed: &ParsedTemplate) -> impl Iterator<Item = &str> {
        parsed.pieces().iter().filter_map(|piece| match piece {
            Piece::Literal(literal) => Some(literal.as_str()),
            _ => None,
        })
    }
}

#[derive(Default)]
struct PathPrefix {
    open_generic_lists: usize,
}

impl PathPrefix {
    const PATH_CHARACTERS: [char; 7] = ['_', ':', '(', '[', ',', ' ', '&'];
    const TURBOFISH_OPENER: &'static str = "::";

    fn extend_with(&mut self, literal: &str) -> bool {
        literal
            .char_indices()
            .all(|(at, character)| match character {
                '<' if literal[..at].ends_with(Self::TURBOFISH_OPENER) => {
                    self.open_generic_lists += 1;
                    true
                }
                '>' if self.open_generic_lists > 0 => {
                    self.open_generic_lists -= 1;
                    true
                }
                _ => character.is_alphanumeric() || Self::PATH_CHARACTERS.contains(&character),
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::emit::test_program::TestProgram;
    use crate::types::{TypedExpr, TypedExprKind};

    struct ShortFixture {
        program: TestProgram,
    }

    impl ShortFixture {
        const SOURCE: &str = "
            (type first-letter (fn ((ref str)) (ref str)))
            (defn first-letter (text) (subs text 0 1))
            (type announced (fn ((ref str)) string))
            (defn announced (text) (str text))
            (type keep (fn (i64 (ref str)) (ref str)))
            (defn keep (n text) text)
            (type seven (fn () i64))
            (defn seven () 7)
            (def lazy-seven (* (seven) 2))
            (type applied (fn ((fn (i64) i64) (ref str)) (ref str)))
            (defn applied (f text) text)
            (def paired (rust \"std::convert::identity(({0}, {1}).1)\" (fn (i64 (ref str)) (ref str))))
            (def after-call (rust \"std::convert::identity((std::process::id(), {0}).1)\" (fn ((ref str)) (ref str))))
            (def logged (rust \"{{ println!(\\\"before\\\"); {0} }}\" (fn ((ref str)) (ref str))))
            (def larger (rust \"std::cmp::max({0}, {0})\" (fn ((ref str)) (ref str))))
            (def anded (rust \"std::convert::identity(({0}, true && false).0)\" (fn ((ref str)) (ref str))))
            (def tried (rust \"std::convert::identity(({0}, Some(1)?).0)\" (fn ((ref str)) (ref str))))
            (def closed (rust \"std::convert::identity(({0}, |x: i32| x).0)\" (fn ((ref str)) (ref str))))
            (def repeated (rust \"std::convert::identity(({0}, [0; 2]).0)\" (fn ((ref str)) (ref str))))
            (defn template-and () (local t (anded (as-str (announced \"a\")))) t)
            (defn template-try () (local t (tried (as-str (announced \"a\")))) t)
            (defn template-closure () (local t (closed (as-str (announced \"a\")))) t)
            (defn template-semicolon () (local t (repeated (as-str (announced \"a\")))) t)
            (defstruct page :title string)
            (defstruct card :page (ref page) :text (ref str))
            (def swapped (rust \"std::convert::identity(({1}, {0}).0)\" (fn ((ref str) i64) (ref str))))
            (defn struct-ahead (held) (local t (->card :page (ref held) :text (as-str (announced \"a\")))) t)
            (defenum labeled (tagged i64 (ref str)))
            (defn variant-ahead (n) (local t (labeled/tagged n (as-str (announced \"a\")))) t)
            (def braced (rust \"{0}.trim_start_matches({{ ' ' }})\" (fn ((ref str)) (ref str))))
            (def compared (rust \"std::convert::identity((1 < 2, {0}).1)\" (fn ((ref str)) (ref str))))
            (def greater (rust \"std::convert::identity((2 > 1, {0}).1)\" (fn ((ref str)) (ref str))))
            (def turbofish (rust \"std::convert::identity::<&str>({0})\" (fn ((ref str)) (ref str))))
            (defn template-less-than () (local t (compared (as-str (announced \"a\")))) t)
            (defn template-greater-than () (local t (greater (as-str (announced \"a\")))) t)
            (defn template-turbofish () (local t (turbofish (as-str (announced \"a\")))) t)
            (defn template-braces () (local t (braced (as-str (announced \"a\")))) t)
            (defn template-callee () (local t ((rust \"std::convert::identity({0})\" (fn ((ref str)) (ref str))) (as-str (announced \"a\")))) t)
            (defn template-reordered () (local t (swapped (as-str (announced \"a\")) (seven))) t)
            (defn function () (local t (first-letter (as-str (announced \"a\")))) t)
            (defn literal-ahead () (local t (keep 1 (as-str (announced \"a\")))) t)
            (defn local-ahead (n) (local t (keep n (as-str (announced \"a\")))) t)
            (defn global-ahead () (local t (keep lazy-seven (as-str (announced \"a\")))) t)
            (defn closure-ahead () (local t (applied (fn (x) x) (as-str (announced \"a\")))) t)
            (defn template-literal-ahead () (local t (paired 1 (as-str (announced \"a\")))) t)
            (defn template-call-ahead () (local t (paired (seven) (as-str (announced \"a\")))) t)
            (defn template-code-ahead () (local t (after-call (as-str (announced \"a\")))) t)
            (defn template-block () (local t (logged (as-str (announced \"a\")))) t)
            (defn template-repeated () (local t (larger (as-str (announced \"a\")))) t)
            (defn local-callee () (local f first-letter) (local t (f (as-str (announced \"a\")))) t)
            (defn branch (flag) (local t (if flag (first-letter (as-str (announced \"a\"))) \"z\")) t)
            (defn main ())";

        fn new() -> Self {
            Self {
                program: TestProgram::new(Self::SOURCE),
            }
        }

        fn admits(&self, function: &str, lent: &[usize]) -> bool {
            let mut path = vec![self.program.last_let_value(function)];
            for position in lent {
                let node = Self::child(path[path.len() - 1], *position);
                path.push(node);
            }
            let plan = self.program.plan();
            TestProgram::short_form(&plan).admits(&path)
        }

        fn child(node: &TypedExpr, position: usize) -> &TypedExpr {
            match &node.kind {
                TypedExprKind::Call(_, arguments) => &arguments[position].value,
                _ => node.children()[position],
            }
        }
    }

    #[test]
    fn admits_a_function_with_nothing_ahead() {
        assert!(ShortFixture::new().admits("function", &[0, 0]));
    }

    #[test]
    fn admits_a_literal_ahead() {
        assert!(ShortFixture::new().admits("literal-ahead", &[1, 0]));
    }

    #[test]
    fn refuses_a_local_ahead() {
        assert!(!ShortFixture::new().admits("local-ahead", &[1, 0]));
    }

    #[test]
    fn refuses_a_global_ahead() {
        assert!(!ShortFixture::new().admits("global-ahead", &[1, 0]));
    }

    #[test]
    fn refuses_a_closure_ahead() {
        assert!(!ShortFixture::new().admits("closure-ahead", &[1, 0]));
    }

    #[test]
    fn admits_a_template_with_a_literal_placeholder_ahead() {
        assert!(ShortFixture::new().admits("template-literal-ahead", &[1, 0]));
    }

    #[test]
    fn refuses_a_template_with_a_call_ahead() {
        assert!(!ShortFixture::new().admits("template-call-ahead", &[1, 0]));
    }

    #[test]
    fn refuses_a_template_that_runs_code_ahead() {
        assert!(!ShortFixture::new().admits("template-code-ahead", &[0, 0]));
    }

    #[test]
    fn refuses_a_template_with_a_block() {
        assert!(!ShortFixture::new().admits("template-block", &[0, 0]));
    }

    #[test]
    fn refuses_a_template_that_repeats_a_placeholder() {
        assert!(!ShortFixture::new().admits("template-repeated", &[0, 0]));
    }

    #[test]
    fn refuses_a_local_callee() {
        assert!(!ShortFixture::new().admits("local-callee", &[0, 0]));
    }

    #[test]
    fn refuses_a_branch() {
        assert!(!ShortFixture::new().admits("branch", &[1, 0, 0]));
    }

    #[test]
    fn refuses_a_template_with_a_lazy_operator() {
        assert!(!ShortFixture::new().admits("template-and", &[0, 0]));
    }

    #[test]
    fn refuses_a_template_that_can_return_early() {
        assert!(!ShortFixture::new().admits("template-try", &[0, 0]));
    }

    #[test]
    fn refuses_a_template_with_a_closure() {
        assert!(!ShortFixture::new().admits("template-closure", &[0, 0]));
    }

    #[test]
    fn refuses_a_template_with_a_semicolon() {
        assert!(!ShortFixture::new().admits("template-semicolon", &[0, 0]));
    }

    #[test]
    fn refuses_a_struct_field_ahead_that_is_not_a_literal() {
        assert!(!ShortFixture::new().admits("struct-ahead", &[1, 0, 0]));
    }

    #[test]
    fn refuses_a_variant_payload_ahead_that_is_not_a_literal() {
        assert!(!ShortFixture::new().admits("variant-ahead", &[1, 0]));
    }

    #[test]
    fn admits_a_template_callee_with_nothing_ahead() {
        assert!(ShortFixture::new().admits("template-callee", &[0, 0]));
    }

    #[test]
    fn refuses_a_placeholder_written_ahead_that_is_not_a_literal() {
        assert!(!ShortFixture::new().admits("template-reordered", &[0, 0]));
    }

    #[test]
    fn admits_a_turbofish_ahead() {
        assert!(ShortFixture::new().admits("template-turbofish", &[0, 0]));
    }

    #[test]
    fn refuses_a_less_than_ahead() {
        assert!(!ShortFixture::new().admits("template-less-than", &[0, 0]));
    }

    #[test]
    fn refuses_a_greater_than_ahead() {
        assert!(!ShortFixture::new().admits("template-greater-than", &[0, 0]));
    }

    #[test]
    fn refuses_a_template_with_braces() {
        assert!(!ShortFixture::new().admits("template-braces", &[0, 0]));
    }
}
