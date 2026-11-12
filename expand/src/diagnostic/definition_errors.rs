use rustc_hash::FxHashMap;

use super::{Diagnostic, DiagnosticCode, Diagnostics, Help, Label};

pub struct DefinitionErrors {
    first_calls: FxHashMap<DefinitionKey, CallPosition>,
}

#[derive(PartialEq, Eq, Hash)]
struct DefinitionKey {
    code: Option<DiagnosticCode>,
    title: String,
    labels: Vec<Label>,
    notes: Vec<String>,
    helps: Vec<Help>,
}

type CallPosition = Option<(usize, usize)>;

impl DefinitionErrors {
    pub fn keep_first_calls(mut diagnostics: Diagnostics) -> Diagnostics {
        let first_calls = Self::of(diagnostics.in_source_order());
        diagnostics.retain(|diagnostic| first_calls.keeps(diagnostic));
        diagnostics
    }

    fn of(diagnostics: Vec<&Diagnostic>) -> Self {
        let mut first_calls: FxHashMap<DefinitionKey, CallPosition> = FxHashMap::default();
        for diagnostic in diagnostics
            .into_iter()
            .filter(|diagnostic| Self::is_of_a_definition(diagnostic))
        {
            let call = Self::call_position(diagnostic);
            first_calls
                .entry(DefinitionKey::of(diagnostic))
                .and_modify(|first| *first = (*first).min(call))
                .or_insert(call);
        }
        Self { first_calls }
    }

    fn keeps(&self, diagnostic: &Diagnostic) -> bool {
        !Self::is_of_a_definition(diagnostic)
            || self.first_calls.get(&DefinitionKey::of(diagnostic))
                == Some(&Self::call_position(diagnostic))
    }

    fn is_of_a_definition(diagnostic: &Diagnostic) -> bool {
        diagnostic.primary().marks_a_definition()
    }

    fn call_position(diagnostic: &Diagnostic) -> CallPosition {
        diagnostic
            .labels()
            .filter(|label| label.marks_a_macro_call())
            .map(|label| (label.span().source().index(), label.span().range().start))
            .min()
    }
}

impl DefinitionKey {
    fn of(diagnostic: &Diagnostic) -> Self {
        Self {
            code: diagnostic.code().cloned(),
            title: diagnostic.title().to_string(),
            labels: diagnostic
                .labels()
                .filter(|label| !label.marks_a_macro_call())
                .cloned()
                .collect(),
            notes: diagnostic.notes().to_vec(),
            helps: diagnostic.helps().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceId, Span};

    struct DefinitionFixture;

    impl DefinitionFixture {
        const DEFINITION: usize = 0;
        const EARLIER_CALL: usize = 10;
        const LATER_CALL: usize = 20;

        fn failure_called_at(call: usize) -> Diagnostic {
            Self::failure_with_primary(call, Label::new(Self::span_at(Self::DEFINITION), "here"))
        }

        fn definition_failure_called_at(call: usize) -> Diagnostic {
            let primary = Label::new(Self::span_at(Self::DEFINITION), "here").in_a_definition();
            Self::failure_with_primary(call, primary)
        }

        fn failure_with_primary(call: usize, primary: Label) -> Diagnostic {
            Diagnostic::error(
                DiagnosticCode::MACRO_FAILED,
                "this macro is not finished",
                primary,
            )
            .with_label(Label::at_macro_call(
                Self::span_at(call),
                "in this macro call",
            ))
        }

        fn span_at(start: usize) -> Span {
            Span::new(SourceId::new(0), start..start + 2)
        }

        fn calls_kept(diagnostics: Vec<Diagnostic>) -> Vec<CallPosition> {
            let kept = DefinitionErrors::keep_first_calls(diagnostics.into_iter().collect());
            kept.in_source_order()
                .into_iter()
                .map(DefinitionErrors::call_position)
                .collect()
        }
    }

    #[test]
    fn a_definition_error_keeps_its_first_call_in_source_order() {
        let later = DefinitionFixture::definition_failure_called_at(DefinitionFixture::LATER_CALL);
        let earlier =
            DefinitionFixture::definition_failure_called_at(DefinitionFixture::EARLIER_CALL);
        assert_eq!(
            DefinitionFixture::calls_kept(vec![later, earlier]),
            [Some((0, DefinitionFixture::EARLIER_CALL))]
        );
    }

    #[test]
    fn a_definition_error_keeps_its_first_call_when_reported_first() {
        let earlier =
            DefinitionFixture::definition_failure_called_at(DefinitionFixture::EARLIER_CALL);
        let later = DefinitionFixture::definition_failure_called_at(DefinitionFixture::LATER_CALL);
        assert_eq!(
            DefinitionFixture::calls_kept(vec![earlier, later]),
            [Some((0, DefinitionFixture::EARLIER_CALL))]
        );
    }

    #[test]
    fn errors_of_a_call_each_stay() {
        let earlier = DefinitionFixture::failure_called_at(DefinitionFixture::EARLIER_CALL);
        let later = DefinitionFixture::failure_called_at(DefinitionFixture::LATER_CALL);
        assert_eq!(DefinitionFixture::calls_kept(vec![earlier, later]).len(), 2);
    }
}
