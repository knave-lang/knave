use std::rc::Rc;

use super::numeric_bound::NumericBound;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct NamespaceProblems;

impl NamespaceProblems {
    pub fn wrong_namespace(name: &str, expected: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::WRONG_NAMESPACE,
            format!("expected {expected}, found `{name}`"),
            Label::new(span, format!("not {expected}")),
        )
    }

    pub fn not_a_trait(name: &str, numeric: Option<NumericBound>, span: Span) -> Diagnostic {
        let diagnostic = Self::wrong_namespace(name, "a trait", span);
        match numeric {
            Some(bound) => diagnostic.with_help(format!(
                "write a trait that every {} type implements, such as {}",
                bound.family(),
                Self::alternatives(bound.traits())
            )),
            None => diagnostic,
        }
    }

    fn alternatives(names: &[Rc<str>]) -> String {
        let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
        match quoted.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::NamespaceProblems;

    struct NamesFixture;

    impl NamesFixture {
        fn joined(names: &[&str]) -> String {
            let names: Vec<Rc<str>> = names.iter().map(|name| Rc::from(*name)).collect();
            NamespaceProblems::alternatives(&names)
        }
    }

    #[test]
    fn one_trait_is_named_alone() {
        assert_eq!(NamesFixture::joined(&["integer"]), "`integer`");
    }

    #[test]
    fn two_traits_join_with_or() {
        assert_eq!(
            NamesFixture::joined(&["integer", "number"]),
            "`integer` or `number`"
        );
    }

    #[test]
    fn three_traits_join_with_commas_then_or() {
        assert_eq!(
            NamesFixture::joined(&["floating", "signed", "number"]),
            "`floating`, `signed` or `number`"
        );
    }
}
