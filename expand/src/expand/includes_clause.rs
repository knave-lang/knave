use super::ExpansionTables;
use super::shape::Shape;
use crate::core::{
    BindingId, RUST_TRAIT_INCLUDES_MARKER, RequiredTrait, TraitRelations, TypeBound,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::reader::Form;
use crate::rust_trait_path::RustTraitPath;
use crate::source::Span;

#[derive(Clone, Copy)]
pub(super) struct IncludedTrait {
    binding: BindingId,
    span: Span,
}

pub(super) struct IncludesEntry<'s, 'f> {
    shape: &'s Shape<'s>,
    entry: &'f Form,
}

impl<'s, 'f> IncludesEntry<'s, 'f> {
    const SECOND_HELP: &'static str = "keep one `includes` clause";
    const HELP: &'static str = "name the one trait whose every implementor this trait includes, as in `(includes display)`";

    pub(super) fn new(shape: &'s Shape<'s>, entry: &'f Form) -> Self {
        Self { shape, entry }
    }

    pub(super) fn lower_into(
        &self,
        included: &mut Option<IncludedTrait>,
        mut lower_bound: impl FnMut(&Form) -> Result<TypeBound, Diagnostic>,
    ) -> Result<bool, Diagnostic> {
        if !self.is_written() {
            return Ok(false);
        }
        if included.is_some() {
            return Err(self
                .shape
                .malformed_with_help(self.entry, Self::SECOND_HELP));
        }
        let Some([_, written]) = self.entry.as_list() else {
            return Err(self.shape.malformed_with_help(self.entry, Self::HELP));
        };
        *included = Some(IncludedTrait {
            binding: lower_bound(written)?.binding,
            span: written.span(),
        });
        Ok(true)
    }

    fn is_written(&self) -> bool {
        let head = self.entry.as_list().and_then(|items| items.first());
        head.and_then(Form::as_symbol)
            .is_some_and(|head| head.name() == RUST_TRAIT_INCLUDES_MARKER)
    }
}

pub(super) struct IncludesClause<'a> {
    binding: BindingId,
    path: &'a str,
}

impl<'a> IncludesClause<'a> {
    pub(super) fn relations(
        requires: Vec<RequiredTrait>,
        included: Option<IncludedTrait>,
    ) -> TraitRelations {
        TraitRelations {
            requires,
            included: included.map(|included| included.binding),
        }
    }

    pub(super) fn new(binding: BindingId, path: &'a str) -> Self {
        Self { binding, path }
    }

    pub(super) fn check_included_path(
        &self,
        included: Option<IncludedTrait>,
        tables: &ExpansionTables,
    ) -> Result<(), Diagnostic> {
        let Some(IncludedTrait {
            binding: included,
            span,
        }) = included
        else {
            return Ok(());
        };
        if tables.trait_paths.get(&included) == Some(&RustTraitPath::from(self.path)) {
            return Ok(());
        }
        let mismatch = IncludedPath {
            name: tables.bindings.get(self.binding).name(),
            included: tables.bindings.get(included).name(),
            path: self.path,
        };
        Err(IncludesProblems::path_mismatch(&mismatch, span))
    }
}

struct IncludedPath<'a> {
    name: &'a str,
    included: &'a str,
    path: &'a str,
}

struct IncludesProblems;

impl IncludesProblems {
    fn path_mismatch(mismatch: &IncludedPath, span: Span) -> Diagnostic {
        let IncludedPath {
            name,
            included,
            path,
        } = mismatch;
        Diagnostic::error(
            DiagnosticCode::INCLUDED_TRAIT_PATH,
            format!("the `includes` clause of `{name}` names `{included}`, which does not bind `{path}`"),
            Label::new(span, format!("this trait does not bind `{path}`")),
        )
        .with_help(format!(
            "name a trait that binds `{path}`, so that a function generic over `{name}` emits the same Rust bound"
        ))
    }
}
