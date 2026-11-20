use super::explanation_sources::ExplanationSources;
use super::field_owner_problems::FieldOwnerProblems;
use super::field_read_selection::FieldReadSelection;
use super::missing_trait::MissingTrait;
use super::selection::Selection;
use super::shown_read_mismatch::ShownReadMismatch;
use crate::diagnostic::Diagnostic;

pub(super) struct FieldReadMismatch<'s, 'a> {
    sources: &'s ExplanationSources<'a>,
}

impl<'s, 'a> FieldReadMismatch<'s, 'a> {
    pub(super) fn new(sources: &'s ExplanationSources<'a>) -> Self {
        Self { sources }
    }

    pub(super) fn explain(&self, missing: &MissingTrait<'_>) -> Option<Diagnostic> {
        let traits = self.sources.declarations.traits();
        let field_traits = traits.rust_paths().field_traits();
        let field = field_traits.bindings().field_of(missing.bound.binding)?;
        let passed = missing.shown_argument.as_ref()?;
        let selection =
            FieldReadSelection::open_selection(traits.table(), missing.bound, missing.implementor);
        let Selection::Implemented {
            argument: Some(read),
            ..
        } = selection
        else {
            return None;
        };
        let display = &self.sources.display;
        let mismatch = ShownReadMismatch {
            owner: display.show(missing.implementor),
            field: field.to_string(),
            read: display.show(&read),
            passed: display.show(passed),
        };
        Some(FieldOwnerProblems::read_mismatch(&mismatch, missing.span))
    }
}
