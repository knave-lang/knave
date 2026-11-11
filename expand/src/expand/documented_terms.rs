use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::noted_terms::NotedTerms;
use super::problems::ExpandProblems;
use super::term_resolver::TermResolver;
use crate::core::{BindingId, ModuleId, Namespace};
use crate::diagnostic::{Diagnostic, SimilarName};
use crate::reader::Symbol;

pub(super) struct DocumentedTerms<'t> {
    resolver: TermResolver<'t>,
    duplicates: &'t FxHashSet<(ModuleId, Rc<str>)>,
    noted: NotedTerms,
}

impl<'t> DocumentedTerms<'t> {
    const TERM_NAMESPACES: [Namespace; 2] = [Namespace::Value, Namespace::Type];

    pub(super) fn new(
        resolver: TermResolver<'t>,
        duplicates: &'t FxHashSet<(ModuleId, Rc<str>)>,
        noted: NotedTerms,
    ) -> Self {
        Self {
            resolver,
            duplicates,
            noted,
        }
    }

    pub(super) fn undefined(&self) -> Vec<Diagnostic> {
        self.noted
            .terms
            .iter()
            .filter(|noted| !self.defines(&noted.term))
            .filter(|noted| !self.reported_as_duplicate(&noted.term))
            .map(|noted| {
                ExpandProblems::undefined_documented_term(
                    noted.term.name(),
                    noted.span,
                    self.closest_own_definition(&noted.term),
                )
            })
            .collect()
    }

    fn reported_as_duplicate(&self, term: &Symbol) -> bool {
        self.duplicates
            .contains(&(self.noted.module, term.shared_name()))
    }

    fn defines(&self, term: &Symbol) -> bool {
        Self::TERM_NAMESPACES
            .into_iter()
            .filter_map(|namespace| self.resolver.resolve(term, namespace))
            .any(|binding| self.is_own(binding))
    }

    fn closest_own_definition(&self, term: &Symbol) -> Option<Rc<str>> {
        if self.resolves(term) {
            return None;
        }
        let own_names: Vec<Rc<str>> = Self::TERM_NAMESPACES
            .into_iter()
            .flat_map(|namespace| self.resolver.visible_bindings(term, namespace))
            .filter(|(_, binding)| self.is_own(*binding))
            .map(|(name, _)| name)
            .collect();
        SimilarName::closest(term.name(), &own_names)
    }

    fn resolves(&self, term: &Symbol) -> bool {
        Self::TERM_NAMESPACES
            .into_iter()
            .any(|namespace| self.resolver.resolve(term, namespace).is_some())
    }

    fn is_own(&self, binding: BindingId) -> bool {
        self.resolver.definition_module(binding) == Some(self.noted.module)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::core::ModulePath;
    use crate::diagnostic::Diagnostic;
    use crate::expand::Expander;
    use crate::modules::ModuleSources;
    use crate::source::{SourceId, SourceMap, Span};

    struct TermsFixture {
        sources: SourceMap,
        entry: SourceId,
    }

    impl TermsFixture {
        const MODULE: &str = "terms";
        const DOCUMENT_USE: &str = "(use prelude/macros document docstring)\n";

        fn new(program: &str) -> Self {
            let mut sources = SourceMap::default();
            let entry = sources.add(Self::MODULE, format!("{}{program}", Self::DOCUMENT_USE));
            Self { sources, entry }
        }

        fn undefined(&mut self) -> Vec<Diagnostic> {
            let entry = self.entry;
            let mut expander = Expander::new(
                &mut self.sources,
                ModuleSources::new(PathBuf::new(), PathBuf::new()),
            );
            let module = expander.load_source_module(ModulePath::new(Self::MODULE), &[entry]);
            expander.documented_terms(module).undefined()
        }

        fn undefined_names(&mut self) -> Vec<String> {
            self.undefined()
                .iter()
                .map(|diagnostic| self.text(diagnostic.primary().span()))
                .collect()
        }

        fn suggestions(&mut self) -> Vec<Option<String>> {
            self.undefined()
                .iter()
                .map(|diagnostic| {
                    diagnostic
                        .helps()
                        .iter()
                        .find_map(|help| help.suggestion())
                        .map(|suggestion| suggestion.edits()[0].replacement().to_string())
                })
                .collect()
        }

        fn text(&self, span: Span) -> String {
            self.sources.file(span.source()).text()[span.range()].to_string()
        }
    }

    #[test]
    fn a_function_the_module_defines_is_documented() {
        let mut fixture =
            TermsFixture::new("(defn twice (x) (* 2 x))\n(document (twice x) \"Doubles.\")\n");
        assert!(fixture.undefined().is_empty());
    }

    #[test]
    fn a_type_the_module_defines_is_documented() {
        let mut fixture =
            TermsFixture::new("(defstruct cell :row i64)\n(document cell \"A cell.\")\n");
        assert!(fixture.undefined().is_empty());
    }

    #[test]
    fn an_unbound_term_is_undefined_at_its_name() {
        let mut fixture = TermsFixture::new("(document (nope) \"Says nothing.\")\n");
        assert_eq!(fixture.undefined_names(), ["nope"]);
    }

    #[test]
    fn a_misspelled_term_suggests_the_closest_own_definition() {
        let mut fixture = TermsFixture::new("(defn greet () 1)\n(document (gret) \"Says hi.\")\n");
        assert_eq!(fixture.suggestions(), [Some("greet".to_string())]);
    }

    #[test]
    fn a_term_bound_by_another_module_suggests_nothing() {
        let mut fixture = TermsFixture::new("(defn printlm () 1)\n(document (println) \"x\")\n");
        assert_eq!(fixture.suggestions(), [None]);
    }

    #[test]
    fn a_definition_that_collides_with_an_import_is_not_undefined() {
        let mut fixture =
            TermsFixture::new("(def docstring (fn () 2))\n(document docstring \"Mine.\")\n");
        assert!(fixture.undefined().is_empty());
    }
}
