use super::collection::Collection;
use super::negative_index_literals::NegativeIndexLiterals;
use super::rust_types::RustTypeTable;
use super::synthetic_template::SyntheticTemplate;
use super::template_signature::TemplateSignature;
use super::templates::TemplateBorrows;
use super::ty::Type;
use super::type_declarations::TypeDeclarations;
use super::typed::{AssignmentPath, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::RustTemplate;
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
            TypedExprKind::Assign(assignment) if assignment.path == AssignmentPath::Elements => {
                NegativeIndexLiterals::in_values(assignment.keys().iter().map(|key| &key.value))
            }
            TypedExprKind::Assoc(_, keys, _) => self.assoc_key_problems(&expr.ty, keys),
            TypedExprKind::Leaf(TypedLeaf::Rust(template))
                if !SyntheticTemplate::builds(template) =>
            {
                self.signature_problems(expr, template)
            }
            _ => Diagnostics::default(),
        }
    }

    fn assoc_key_problems(&self, ty: &Type, keys: &[TypedExpr]) -> Diagnostics {
        Collection::of(ty, self.rust_types)
            .and_then(|collection| collection.key_conversion())
            .filter(|conversion| conversion.rejects_negative_literal())
            .map_or_else(Diagnostics::default, |_| {
                NegativeIndexLiterals::in_values(keys)
            })
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
