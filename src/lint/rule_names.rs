use super::problems::LintProblems;
use super::rule_program::RuleProgram;
use crate::diagnostic::Diagnostic;
use crate::interpret::Value;
use crate::reader::Form;
use crate::source::Span;

pub struct RuleNames;

impl RuleNames {
    pub fn of(value: &Value, span: Span) -> Result<Vec<Form>, Diagnostic> {
        let names = value
            .to_form()
            .ok()
            .flatten()
            .and_then(|list| list.as_list().map(<[Form]>::to_vec))
            .filter(|names| names.iter().all(|name| name.as_symbol().is_some()));
        names.ok_or_else(|| LintProblems::malformed_rule_names(RuleProgram::RULES, span))
    }
}

#[cfg(test)]
mod tests {
    use super::RuleNames;
    use crate::diagnostic::DiagnosticCode;
    use crate::interpret::Value;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct NamesFixture;

    impl NamesFixture {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn form(kind: FormKind) -> Form {
            Form::new(kind, Self::span())
        }

        fn names(items: Vec<Form>) -> Value {
            Value::Form(Self::form(FormKind::List(items)))
        }

        fn symbol(name: &str) -> Form {
            Self::form(FormKind::Symbol(Symbol::new(name)))
        }
    }

    #[test]
    fn a_list_of_symbols_names_the_rules() {
        let value = NamesFixture::names(vec![NamesFixture::symbol("unused-binding")]);
        let names = RuleNames::of(&value, NamesFixture::span()).expect("the names read");
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn a_name_that_is_no_symbol_is_reported() {
        let value = NamesFixture::names(vec![NamesFixture::form(FormKind::String("x".into()))]);
        let failure =
            RuleNames::of(&value, NamesFixture::span()).expect_err("a string names no rule");
        assert_eq!(failure.code(), Some(&DiagnosticCode::COMPILE_TIME_ERROR));
    }

    #[test]
    fn a_value_that_is_no_list_is_reported() {
        let value = Value::Form(NamesFixture::symbol("unused-binding"));
        assert!(RuleNames::of(&value, NamesFixture::span()).is_err());
    }
}
