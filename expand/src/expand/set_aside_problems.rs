use super::set_aside_input::SetAsideInput;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct SetAsideProblems;

impl SetAsideProblems {
    const LISTED_NAMES: usize = 3;
    const SET_ASIDE_LABEL: &'static str = "set aside";

    pub fn set_aside(input: &SetAsideInput<'_>, error: &Diagnostic) -> Diagnostic {
        let primary = error.primary();
        let warning = Diagnostic::warning(
            DiagnosticCode::INPUT_SET_ASIDE,
            Self::title(input),
            Label::new(primary.span(), primary.message()),
        );
        let labeled = error
            .labels()
            .skip(1)
            .fold(warning, |warning, label| warning.with_label(label.clone()));
        let noted = error.notes().iter().fold(
            labeled.with_note(Self::error_line(error)),
            |warning, note| warning.with_note(note.clone()),
        );
        let helped = Self::with_helps_of(noted, error).with_help(Self::enter_again(input));
        match error.missing_name() {
            Some(missing) => helped.with_missing_name(missing.clone()),
            None => helped,
        }
    }

    pub fn dropped_definitions(
        input: &SetAsideInput<'_>,
        replacing: &str,
        span: Span,
    ) -> Diagnostic {
        Self::dropped(input, span)
            .with_note(format!(
                "`{replacing}` replaced the input that defined {}",
                Self::listed_names(input)
            ))
            .with_help(Self::enter_again(input))
    }

    pub fn dropped_documents(input: &SetAsideInput<'_>, replacing: &str, span: Span) -> Diagnostic {
        let listed = Self::listed_names(input);
        Self::dropped(input, span)
            .with_note(format!(
                "a `document` of `{replacing}` replaced the input that documented {listed}"
            ))
            .with_help(format!("enter the `document` of {listed} again to keep it"))
    }

    pub fn set_aside_reason(replacing: &str, dropped: &str) -> String {
        format!("`{replacing}` replaced the input that defined `{dropped}`")
    }

    fn dropped(input: &SetAsideInput<'_>, span: Span) -> Diagnostic {
        Diagnostic::warning(
            DiagnosticCode::INPUT_SET_ASIDE,
            Self::title(input),
            Label::new(span, Self::SET_ASIDE_LABEL),
        )
    }

    fn title(input: &SetAsideInput<'_>) -> String {
        format!(
            "set aside {} after `{}`",
            Self::listed_names(input),
            input.setter
        )
    }

    fn enter_again(input: &SetAsideInput<'_>) -> String {
        let pronoun = if input.names.len() > 1 { "them" } else { "it" };
        format!("enter {} again to use {pronoun}", Self::listed_names(input))
    }

    fn listed_names(input: &SetAsideInput<'_>) -> String {
        let quoted: Vec<String> = input.names.iter().map(|name| format!("`{name}`")).collect();
        match quoted.as_slice() {
            [] => format!("`{}`", input.source),
            [only] => only.clone(),
            shown if shown.len() <= Self::LISTED_NAMES => {
                let (last, rest) = shown.split_last().expect("two names or more");
                format!("{} and {last}", rest.join(", "))
            }
            more => format!(
                "{} and {} more",
                more[..Self::LISTED_NAMES].join(", "),
                more.len() - Self::LISTED_NAMES
            ),
        }
    }

    fn error_line(error: &Diagnostic) -> String {
        let code = error.code().expect("an error has a code");
        format!("error[{}]: {}", code.as_str(), error.title())
    }

    fn with_helps_of(warning: Diagnostic, error: &Diagnostic) -> Diagnostic {
        let helps = error.helps().iter().cloned();
        helps.fold(warning, Diagnostic::with_help)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceId;

    struct SetAsideFixture;

    impl SetAsideFixture {
        const SOURCE: &str = "<input 2>";
        const SETTER: &str = "<input 4>";

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..4)
        }

        fn input<'a>(names: &'a [&'a str]) -> SetAsideInput<'a> {
            SetAsideInput {
                names,
                source: Self::SOURCE,
                setter: Self::SETTER,
            }
        }

        fn title(names: &[&str]) -> String {
            SetAsideProblems::dropped_definitions(&Self::input(names), "g", Self::span())
                .title()
                .to_string()
        }

        fn note_and_help(diagnostic: &Diagnostic) -> (&str, &str) {
            (&diagnostic.notes()[0], diagnostic.helps()[0].message())
        }
    }

    #[test]
    fn an_input_that_bound_no_name_is_named_by_its_source() {
        assert_eq!(
            SetAsideFixture::title(&[]),
            "set aside `<input 2>` after `<input 4>`"
        );
    }

    #[test]
    fn the_title_lists_one_name() {
        assert_eq!(
            SetAsideFixture::title(&["karl"]),
            "set aside `karl` after `<input 4>`"
        );
    }

    #[test]
    fn the_title_lists_three_names() {
        assert_eq!(
            SetAsideFixture::title(&["a", "b", "c"]),
            "set aside `a`, `b` and `c` after `<input 4>`"
        );
    }

    #[test]
    fn the_title_counts_the_names_past_three() {
        assert_eq!(
            SetAsideFixture::title(&["a", "b", "c", "d", "e"]),
            "set aside `a`, `b`, `c` and 2 more after `<input 4>`"
        );
    }

    #[test]
    fn dropped_definitions_say_what_defined_them() {
        let names = ["k"];
        let warning = SetAsideProblems::dropped_definitions(
            &SetAsideFixture::input(&names),
            "g",
            SetAsideFixture::span(),
        );
        assert_eq!(
            SetAsideFixture::note_and_help(&warning),
            (
                "`g` replaced the input that defined `k`",
                "enter `k` again to use it"
            )
        );
    }

    #[test]
    fn dropped_documents_say_what_documented_them() {
        let terms = ["k"];
        let warning = SetAsideProblems::dropped_documents(
            &SetAsideFixture::input(&terms),
            "g",
            SetAsideFixture::span(),
        );
        assert_eq!(
            SetAsideFixture::note_and_help(&warning),
            (
                "a `document` of `g` replaced the input that documented `k`",
                "enter the `document` of `k` again to keep it"
            )
        );
    }

    #[test]
    fn a_set_aside_input_keeps_the_quoted_form_of_a_help() {
        let span = SetAsideFixture::span();
        let error = Diagnostic::error(
            DiagnosticCode::UNBOUND_NAME,
            "a title",
            Label::new(span, "a label"),
        )
        .with_quoting_help("bind it: `{form}`", span);
        let names = ["karl"];
        let warning = SetAsideProblems::set_aside(&SetAsideFixture::input(&names), &error);
        assert_eq!(warning.helps()[0].quoted_form(), Some(span));
    }
}
