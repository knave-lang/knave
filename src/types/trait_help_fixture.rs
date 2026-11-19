use super::declarations_fixture::DeclarationsFixture;
use super::explanation_sources::ExplanationSources;
use super::problems::TypeProblems;
use super::ty::Bound;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::{SourceId, Span};

pub(super) struct TraitHelpFixture {
    declarations: DeclarationsFixture,
}

impl TraitHelpFixture {
    pub(super) fn new(source: &str) -> Self {
        Self {
            declarations: DeclarationsFixture::new(source),
        }
    }

    pub(super) fn explain(
        &self,
        type_name: &str,
        trait_name: &str,
        add_help: impl Fn(BindingId, &Bound, &ExplanationSources<'_>, Diagnostic) -> Diagnostic,
    ) -> Diagnostic {
        let binding = self.declarations.type_named(type_name);
        let bound = Bound::plain(self.declarations.trait_named(trait_name));
        let sources = self.declarations.explanation_sources();
        add_help(binding, &bound, &sources, Self::unexplained(trait_name))
    }

    pub(super) fn added_helps<'d>(diagnostic: &'d Diagnostic, trait_name: &str) -> Vec<&'d str> {
        let unexplained = Self::unexplained(trait_name).helps().len();
        (diagnostic.helps().iter().skip(unexplained))
            .map(|help| help.message())
            .collect()
    }

    fn unexplained(trait_name: &str) -> Diagnostic {
        TypeProblems::missing_bound(trait_name, "T", Span::new(SourceId::new(0), 0..0))
    }
}
