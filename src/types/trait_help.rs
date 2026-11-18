use super::RustTrait;
use super::explanation_sources::ExplanationSources;
use super::ty::Bound;
use crate::diagnostic::Diagnostic;

pub(super) trait TraitHelp {
    fn needed(&self) -> &Bound;

    fn with_order_help(
        &self,
        diagnostic: Diagnostic,
        sources: &ExplanationSources<'_>,
    ) -> Diagnostic;

    fn with_other_help(
        &self,
        diagnostic: Diagnostic,
        sources: &ExplanationSources<'_>,
    ) -> Diagnostic;

    fn add_to(&self, diagnostic: Diagnostic, sources: &ExplanationSources<'_>) -> Diagnostic {
        let traits = &sources.declarations.traits().table;
        if traits
            .get(self.needed().binding)
            .is_some_and(RustTrait::is_order)
        {
            self.with_order_help(diagnostic, sources)
        } else {
            self.with_other_help(diagnostic, sources)
        }
    }
}
