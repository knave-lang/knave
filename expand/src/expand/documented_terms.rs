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
