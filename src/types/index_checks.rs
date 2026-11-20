use super::collection::Collection;
use super::negative_index_literals::NegativeIndexLiterals;
use super::rust_types::RustTypeTable;
use super::template_signature::TemplateSignature;
use super::templates::TemplateBorrows;
use super::ty::Type;
use super::type_declarations::TypeDeclarations;
use super::typed::{TypedArgument, TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{RustTemplate, TypeExprKind};
use crate::diagnostic::Diagnostics;

pub(super) struct IndexChecks<'t> {
    templates: &'t TemplateBorrows,
    rust_types: &'t RustTypeTable,
}

impl<'t> IndexChecks<'t> {
    pub(super) fn new(templates: &'t TemplateBorrows, types: &'t TypeDeclarations) -> Self {
        Self {
            templates,
            rust_types: &types.rust_types,
        }
    }

    pub(super) fn problems(&self, expr: &TypedExpr) -> Diagnostics {
        match &expr.kind {
            TypedExprKind::Call(callee, arguments) => self.negative_literals(callee, arguments),
            TypedExprKind::Assoc(_, keys, _) if self.indexes_a_sequence(&expr.ty) => {
                NegativeIndexLiterals::in_values(keys)
            }
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) if Self::has_written_type(template) => {
                self.signature_problems(expr, template)
            }
            _ => Diagnostics::default(),
        }
    }

    fn indexes_a_sequence(&self, ty: &Type) -> bool {
        Collection::of(ty, self.rust_types).is_some_and(|collection| !collection.is_map())
    }

    fn has_written_type(template: &RustTemplate) -> bool {
        !matches!(template.type_expr.kind(), TypeExprKind::Hole)
    }

    fn negative_literals(&self, callee: &TypedExpr, arguments: &[TypedArgument]) -> Diagnostics {
        self.templates
            .conversions(callee)
            .map_or_else(Diagnostics::default, |conversions| {
                NegativeIndexLiterals::new(&conversions).problems(arguments)
            })
    }

    fn signature_problems(&self, leaf: &TypedExpr, template: &RustTemplate) -> Diagnostics {
        self.templates
            .conversions(leaf)
            .map_or_else(Diagnostics::default, |conversions| {
                TemplateSignature::new(template, self.rust_types.option_binding(), leaf.span)
                    .index_problems(&conversions)
            })
    }
}
