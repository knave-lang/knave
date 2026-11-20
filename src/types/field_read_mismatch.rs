use super::explanation_sources::ExplanationSources;
use super::field_owner_problems::FieldOwnerProblems;
use super::missing_trait::MissingTrait;
use super::selection::Selection;
use super::ty::{Bound, Type};
use super::unborrowed::Unborrowed;
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
        let read = self.struct_read(missing.bound, missing.implementor)?;
        let display = &self.sources.display;
        let owner = display.show(missing.implementor);
        let reads = (display.show(&read), display.show(passed));
        Some(FieldOwnerProblems::read_mismatch(
            (&owner, field),
            reads,
            missing.span,
        ))
    }

    fn struct_read(&self, bound: &Bound, implementor: &Type) -> Option<Type> {
        let open = Bound {
            argument: None,
            ..bound.clone()
        };
        let rows = self
            .sources
            .declarations
            .traits()
            .table()
            .rows(bound.binding);
        match rows.select(&open, Unborrowed::of(implementor)) {
            Selection::Implemented { argument, .. } => argument,
            _ => None,
        }
    }
}
