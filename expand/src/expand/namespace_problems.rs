use std::rc::Rc;

use super::numeric_bound::NumericBound;
use crate::core::KernelType;
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
                Self::alternatives(bound.first(), bound.others())
            )),
            None => diagnostic,
        }
    }

    pub fn kernel_type_defined(written: &str, hidden: KernelType, span: Span) -> Diagnostic {
        let label = Label::new(
            span,
            format!("this binds `{written}` in place of the kernel type"),
        );
        Self::kernel_type_hidden(written, hidden, label)
            .with_help(format!("rename this `{written}`"))
    }

    pub fn kernel_type_imported(written: &str, hidden: KernelType, span: Span) -> Diagnostic {
        let label = Label::new(
            span,
            format!("this imports `{written}` in place of the kernel type"),
        );
        Self::kernel_type_hidden(written, hidden, label).with_help(format!(
            "leave `{written}` out of this `use`, or rename it in the module that defines it"
        ))
    }

    fn kernel_type_hidden(written: &str, hidden: KernelType, label: Label) -> Diagnostic {
        let shown = hidden.name();
        Diagnostic::warning(
            DiagnosticCode::KERNEL_TYPE_HIDDEN,
            format!("`{written}` hides the kernel type `{shown}`"),
            label,
        )
        .with_note(format!(
            "in this scope, a type written `{written}` names this binding, while a printed `{shown}` is the kernel type"
        ))
    }

    fn alternatives(first: &str, others: &[Rc<str>]) -> String {
        match others.split_last() {
            None => format!("`{first}`"),
            Some((last, middle)) => {
                let leading: Vec<String> = std::iter::once(first)
                    .chain(middle.iter().map(|name| name.as_ref()))
                    .map(|name| format!("`{name}`"))
                    .collect();
                format!("{} or `{last}`", leading.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::NamespaceProblems;

    struct NamesFixture;

    impl NamesFixture {
        fn joined(first: &str, others: &[&str]) -> String {
            let others: Vec<Rc<str>> = others.iter().map(|name| Rc::from(*name)).collect();
            NamespaceProblems::alternatives(first, &others)
        }
    }

    #[test]
    fn one_trait_is_named_alone() {
        assert_eq!(NamesFixture::joined("integer", &[]), "`integer`");
    }

    #[test]
    fn two_traits_join_with_or() {
        assert_eq!(
            NamesFixture::joined("integer", &["number"]),
            "`integer` or `number`"
        );
    }

    #[test]
    fn three_traits_join_with_commas_then_or() {
        assert_eq!(
            NamesFixture::joined("floating", &["signed", "number"]),
            "`floating`, `signed` or `number`"
        );
    }
}
