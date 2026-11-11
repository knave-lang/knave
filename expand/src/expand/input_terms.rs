use std::rc::Rc;

use super::recorded_inputs::BoundName;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputTerms {
    bound: Vec<BoundName>,
    documented: Vec<Rc<str>>,
}

pub struct DocumentsLeft {
    pub replacing: Rc<str>,
    pub left: Vec<Rc<str>>,
}

impl InputTerms {
    pub fn binding(bound: Vec<BoundName>) -> Self {
        Self::documenting(bound, Vec::new())
    }

    pub fn documenting(bound: Vec<BoundName>, documented: Vec<Rc<str>>) -> Self {
        Self { bound, documented }
    }

    pub fn bound(&self) -> &[BoundName] {
        &self.bound
    }

    pub fn rebound(&self, bound: Vec<BoundName>) -> Self {
        Self::documenting(bound, self.documented.clone())
    }

    pub fn binds(&self, name: &BoundName) -> bool {
        self.bound.iter().any(|bound| bound.names_the_same(name))
    }

    pub fn documents_left_by(&self, later: &InputTerms) -> Option<DocumentsLeft> {
        let replacing = later
            .documented
            .iter()
            .find(|term| self.documented.contains(term))?;
        let left: Vec<Rc<str>> = self
            .documented
            .iter()
            .filter(|term| !later.documented.contains(term))
            .cloned()
            .collect();
        Some(DocumentsLeft {
            replacing: Rc::clone(replacing),
            left,
        })
    }

    pub fn shares_a_term_with(&self, other: &InputTerms) -> bool {
        let shares_a_name = self.bound.iter().any(|name| other.binds(name));
        let documents_a_term_again = self
            .documented
            .iter()
            .any(|term| other.documented.contains(term));
        shares_a_name || documents_a_term_again
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Namespace;
    use crate::source::{SourceId, Span};

    struct TermsFixture;

    impl TermsFixture {
        fn name(name: &str) -> BoundName {
            BoundName {
                name: name.into(),
                namespace: Namespace::Value,
                span: Span::new(SourceId::new(0), 0..0),
            }
        }

        fn binding(names: &[&str]) -> InputTerms {
            InputTerms::binding(names.iter().map(|name| Self::name(name)).collect())
        }

        fn documenting(terms: &[&str]) -> InputTerms {
            InputTerms::documenting(
                Vec::new(),
                terms.iter().map(|term| Rc::from(*term)).collect(),
            )
        }
    }

    #[test]
    fn terms_bind_only_their_bound_names() {
        let terms = TermsFixture::binding(&["a"]);
        assert!(terms.binds(&TermsFixture::name("a")));
        assert!(!terms.binds(&TermsFixture::name("b")));
        assert_eq!(terms.bound(), [TermsFixture::name("a")]);
    }

    #[test]
    fn terms_that_bind_a_shared_name_share_a_term() {
        let earlier = TermsFixture::binding(&["a", "b"]);
        assert!(earlier.shares_a_term_with(&TermsFixture::binding(&["b"])));
        assert!(!earlier.shares_a_term_with(&TermsFixture::binding(&["c"])));
    }

    #[test]
    fn terms_that_document_a_shared_term_share_a_term() {
        let earlier = TermsFixture::documenting(&["g", "k"]);
        assert!(earlier.shares_a_term_with(&TermsFixture::documenting(&["k"])));
        assert!(!earlier.shares_a_term_with(&TermsFixture::documenting(&["h"])));
    }

    #[test]
    fn a_name_bound_and_a_term_documented_are_not_shared() {
        let binding = TermsFixture::binding(&["g"]);
        let documenting = TermsFixture::documenting(&["g"]);
        assert!(!binding.shares_a_term_with(&documenting));
        assert!(!documenting.shares_a_term_with(&binding));
    }

    #[test]
    fn a_later_document_of_one_term_leaves_the_other_terms_behind() {
        let earlier = TermsFixture::documenting(&["g", "k"]);
        let left = earlier
            .documents_left_by(&TermsFixture::documenting(&["g"]))
            .expect("the later input documents `g` again");
        assert_eq!(&*left.replacing, "g");
        assert_eq!(left.left, [Rc::from("k")]);
    }

    #[test]
    fn a_later_document_of_another_term_leaves_nothing() {
        let earlier = TermsFixture::documenting(&["g"]);
        assert!(
            earlier
                .documents_left_by(&TermsFixture::documenting(&["h"]))
                .is_none()
        );
    }

    #[test]
    fn rebound_terms_keep_what_they_document() {
        let rebound = TermsFixture::documenting(&["g"]).rebound(vec![TermsFixture::name("a")]);
        assert_eq!(rebound.bound(), [TermsFixture::name("a")]);
        assert!(rebound.shares_a_term_with(&TermsFixture::documenting(&["g"])));
    }
}
