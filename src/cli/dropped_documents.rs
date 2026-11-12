use std::rc::Rc;

use super::name_records::NameRecords;
use super::set_aside_drop::SetAsideDrop;
use super::set_aside_source::SetAsideSource;
use crate::diagnostic::Diagnostic;
use crate::expand::{InputTerms, SetAsideInput, SetAsideProblems};

pub struct DroppedDocuments {
    source: SetAsideSource,
    documents: Vec<Rc<str>>,
    replacing: Rc<str>,
}

impl DroppedDocuments {
    pub fn left_behind(
        source: SetAsideSource,
        earlier: &InputTerms,
        entered: &InputTerms,
    ) -> Option<Self> {
        let again = entered.documented();
        let replacing = again
            .iter()
            .find(|term| earlier.documented().contains(term))?;
        let left: Vec<Rc<str>> = earlier
            .documented()
            .iter()
            .filter(|term| !again.contains(term))
            .cloned()
            .collect();
        (!left.is_empty()).then(|| Self {
            source,
            documents: left,
            replacing: Rc::clone(replacing),
        })
    }
}

impl SetAsideDrop for DroppedDocuments {
    fn warning(&self, setter: &str) -> Diagnostic {
        let terms: Vec<&str> = self.documents.iter().map(|term| &**term).collect();
        let input = SetAsideInput {
            names: &terms,
            source: &self.source.name,
            setter,
        };
        SetAsideProblems::dropped_documents(&input, &self.replacing, self.source.span)
    }

    fn record(&self, _records: &mut NameRecords, _setter: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::name_records::Tense;
    use crate::core::Namespace;
    use crate::diagnostic::MissingName;
    use crate::expand::InputTerms;
    use crate::source::{SourceId, Span};

    struct DocumentsFixture;

    impl DocumentsFixture {
        const SOURCE: &str = "<input 2>";
        const SETTER: &str = "<input 3>";

        fn documenting(terms: &[&str]) -> InputTerms {
            InputTerms::documenting(
                Vec::new(),
                terms.iter().map(|term| Rc::from(*term)).collect(),
            )
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn left_behind(earlier: &[&str], later: &[&str]) -> Option<DroppedDocuments> {
            let source = SetAsideSource {
                name: Self::SOURCE.to_string(),
                span: Self::span(),
            };
            DroppedDocuments::left_behind(
                source,
                &Self::documenting(earlier),
                &Self::documenting(later),
            )
        }
    }

    #[test]
    fn a_document_of_another_term_drops_nothing() {
        assert!(DocumentsFixture::left_behind(&["g"], &["h"]).is_none());
    }

    #[test]
    fn the_replacing_term_is_the_one_documented_again() {
        let dropped = DocumentsFixture::left_behind(&["g", "k"], &["h", "g"]).expect("`k` is left");
        let warning = dropped.warning(DocumentsFixture::SETTER);
        assert_eq!(
            warning.notes(),
            ["a `document` of `g` replaced the input that documented `k`"]
        );
    }

    #[test]
    fn a_document_of_every_term_again_leaves_nothing_behind() {
        assert!(DocumentsFixture::left_behind(&["g"], &["g"]).is_none());
    }

    #[test]
    fn the_warning_names_the_documents_left_behind_at_the_earlier_input() {
        let dropped = DocumentsFixture::left_behind(&["g", "k"], &["g"]).expect("`k` is left");
        let warning = dropped.warning(DocumentsFixture::SETTER);
        assert_eq!(warning.title(), "set aside `k` after `<input 3>`");
        assert_eq!(warning.primary().span(), DocumentsFixture::span());
    }

    #[test]
    fn a_document_left_behind_records_no_name() {
        let dropped = DocumentsFixture::left_behind(&["g", "k"], &["g"]).expect("`k` is left");
        let mut records = NameRecords::default();
        dropped.record(&mut records, DocumentsFixture::SETTER);
        let missing = MissingName {
            name: "k".into(),
            namespace: Namespace::Value,
        };
        assert_eq!(records.note(&missing, Tense::Done), None);
    }
}
