use super::result_borrow_source::ResultBorrowSource;
use super::trait_declarations::TraitDeclarations;
use super::typed::{TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::TypeExpr;
use crate::diagnostic::Diagnostic;

pub(super) struct TemplateResultBorrows<'t> {
    traits: &'t TraitDeclarations,
}

impl<'t> TemplateResultBorrows<'t> {
    pub(super) fn new(traits: &'t TraitDeclarations) -> Self {
        Self { traits }
    }

    pub(super) fn inline_problem(&self, leaf: &TypedExpr) -> Option<Diagnostic> {
        let TypedExprKind::Leaf(TypedLeaf::Rust(template)) = &leaf.kind else {
            return None;
        };
        Self::problem(&template.type_expr)
    }

    pub(super) fn declared_problem(&self, definition: &TypedDefinition) -> Option<Diagnostic> {
        let TypedExprKind::Leaf(TypedLeaf::Rust(template)) = &definition.value.kind else {
            return None;
        };
        if self.traits.methods().contains_key(&definition.binding) {
            return None;
        }
        Self::problem(&template.type_expr)
    }

    fn problem(written: &TypeExpr) -> Option<Diagnostic> {
        ResultBorrowSource::of(written)?.check().err()
    }
}
