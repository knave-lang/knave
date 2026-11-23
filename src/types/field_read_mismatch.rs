use super::explanation_sources::ExplanationSources;
use super::field_owner_problems::FieldOwnerProblems;
use super::field_read_selection::FieldReadSelection;
use super::inferred_at::InferredAt;
use super::inferred_bound_problems::InferredBoundProblems;
use super::missing_trait::MissingTrait;
use super::selection::Selection;
use super::shown_read_mismatch::ShownReadMismatch;
use super::ty::Type;
use super::unborrowed::Unborrowed;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

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
        let inferred = (InferredAt::of(&missing.bound.origins).first())
            .map(|read| InferredBoundProblems::new(field, read));
        let selection =
            FieldReadSelection::open_selection(traits.table(), missing.bound, missing.implementor);
        match (selection, inferred) {
            (
                Selection::Implemented {
                    argument: Some(read),
                    ..
                },
                inferred,
            ) => {
                let mismatch = self.shown_mismatch(missing, field, read)?;
                Some(Self::read_mismatch(&mismatch, inferred, missing.span))
            }
            (Selection::Missing, Some(problems)) => Some(self.lacking_field(missing, &problems)),
            _ => None,
        }
    }

    fn shown_mismatch(
        &self,
        missing: &MissingTrait<'_>,
        field: &str,
        read: Type,
    ) -> Option<ShownReadMismatch> {
        let bound = missing.shown_argument.as_ref()?;
        let display = &self.sources.display;
        Some(ShownReadMismatch {
            owner: display.show(missing.implementor),
            field: field.to_string(),
            read: display.show(&read),
            bound: display.show(bound),
        })
    }

    fn read_mismatch(
        mismatch: &ShownReadMismatch,
        inferred: Option<InferredBoundProblems>,
        span: Span,
    ) -> Diagnostic {
        match inferred {
            Some(problems) => problems.read_mismatch(mismatch, span),
            None => FieldOwnerProblems::read_mismatch(mismatch, span),
        }
    }

    fn lacking_field(
        &self,
        missing: &MissingTrait<'_>,
        problems: &InferredBoundProblems,
    ) -> Diagnostic {
        let shown = self
            .sources
            .display
            .show(Unborrowed::of(missing.implementor));
        let structs = &self.sources.declarations.types().structs;
        match structs.behind(missing.implementor) {
            Some(_) => problems.lacking_field(&shown, missing.span),
            None => problems.no_fields(&shown, missing.span),
        }
    }
}
