use std::rc::Rc;

use super::duplicate_definitions::DuplicateDefinitions;
use super::noted_term::NotedTerm;
use crate::core::ModuleId;
use crate::reader::Symbol;
use crate::source::Span;

pub(super) struct NotedTerms {
    module: ModuleId,
    terms: Vec<NotedTerm>,
}

impl NotedTerms {
    pub(super) fn new(module: ModuleId, terms: Vec<NotedTerm>) -> Self {
        Self { module, terms }
    }

    pub(super) fn named_within(&self, span: Span) -> Vec<Rc<str>> {
        self.terms
            .iter()
            .filter(|noted| span.contains(noted.span))
            .map(|noted| noted.term.shared_name())
            .collect()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &NotedTerm> {
        self.terms.iter()
    }

    pub(super) fn belongs_to(&self, module: Option<ModuleId>) -> bool {
        module == Some(self.module)
    }

    pub(super) fn is_duplicate(&self, term: &Symbol, duplicates: &DuplicateDefinitions) -> bool {
        duplicates.holds(self.module, term)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::*;
    use crate::source::SourceId;

    struct NotedFixture;

    impl NotedFixture {
        const MODULE_INDEX: usize = 3;
        const OTHER_MODULE_INDEX: usize = 4;
        const FIRST: &str = "greet";
        const SECOND: &str = "gret";
        const SECOND_START: usize = 10;

        fn noted(name: &str, start: usize) -> NotedTerm {
            NotedTerm {
                term: Symbol::new(name),
                span: Self::span(start..start + name.len()),
            }
        }

        fn module() -> ModuleId {
            ModuleId::new(Self::MODULE_INDEX)
        }

        fn span(range: Range<usize>) -> Span {
            Span::new(SourceId::new(0), range)
        }

        fn terms() -> NotedTerms {
            NotedTerms::new(
                Self::module(),
                vec![
                    Self::noted(Self::FIRST, 0),
                    Self::noted(Self::SECOND, Self::SECOND_START),
                ],
            )
        }
    }

    #[test]
    fn only_the_terms_noted_within_a_span_are_named_within_it() {
        let within = NotedFixture::span(NotedFixture::SECOND_START..NotedFixture::SECOND_START + 8);
        let names = NotedFixture::terms().named_within(within);
        assert_eq!(names, [Rc::from(NotedFixture::SECOND)]);
    }

    #[test]
    fn the_terms_iterate_in_the_order_they_were_noted() {
        let terms = NotedFixture::terms();
        let names: Vec<&str> = terms.iter().map(|noted| noted.term.name()).collect();
        assert_eq!(names, [NotedFixture::FIRST, NotedFixture::SECOND]);
    }

    #[test]
    fn the_terms_belong_only_to_their_own_module() {
        let terms = NotedFixture::terms();
        assert!(terms.belongs_to(Some(NotedFixture::module())));
        assert!(!terms.belongs_to(Some(ModuleId::new(NotedFixture::OTHER_MODULE_INDEX))));
        assert!(!terms.belongs_to(None));
    }

    #[test]
    fn a_term_is_a_duplicate_only_where_its_module_holds_it() {
        let term = Symbol::new(NotedFixture::FIRST);
        let mut own = DuplicateDefinitions::default();
        own.remember(NotedFixture::module(), &term);
        let mut other = DuplicateDefinitions::default();
        other.remember(ModuleId::new(NotedFixture::OTHER_MODULE_INDEX), &term);
        assert!(NotedFixture::terms().is_duplicate(&term, &own));
        assert!(!NotedFixture::terms().is_duplicate(&term, &other));
    }
}
