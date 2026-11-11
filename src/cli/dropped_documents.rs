use std::rc::Rc;

use super::name_records::NameRecords;
use super::set_aside_drop::SetAsideDrop;
use crate::diagnostic::Diagnostic;
use crate::expand::{DocumentsLeft, SetAsideInput, SetAsideProblems};
use crate::source::Span;

pub struct DroppedDocuments {
    source: String,
    span: Span,
    documents: Vec<Rc<str>>,
    replacing: Rc<str>,
}

impl DroppedDocuments {
    pub fn left_behind(source: &str, span: Span, documents: DocumentsLeft) -> Option<Self> {
        let (replacing, left) = documents.into_parts();
        (!left.is_empty()).then(|| Self {
            source: source.to_string(),
            span,
            documents: left,
            replacing,
        })
    }
}

impl SetAsideDrop for DroppedDocuments {
    fn warning(&self, setter: &str) -> Diagnostic {
        let terms: Vec<&str> = self.documents.iter().map(|term| &**term).collect();
        let input = SetAsideInput {
            names: &terms,
            source: &self.source,
            setter,
        };
        SetAsideProblems::dropped_documents(&input, &self.replacing, self.span)
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
    use crate::source::SourceId;

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
            let documents = Self::documenting(earlier)
                .documents_left_by(&Self::documenting(later))
                .expect("the later input documents a term again");
            DroppedDocuments::left_behind(Self::SOURCE, Self::span(), documents)
        }
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
