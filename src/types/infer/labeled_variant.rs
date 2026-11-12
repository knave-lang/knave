use std::rc::Rc;

use crate::core::{BindingId, Expr, FieldLabel};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::checker::TypeChecker;
use crate::types::problems::{MismatchContext, TypeProblems};
use crate::types::{FieldOrder, Type, TypedExpr, TypedExprKind};

impl TypeChecker {
    pub(super) fn infer_labeled_variant(
        &mut self,
        binding: BindingId,
        fields: &[(FieldLabel, Expr)],
        expected: Option<&Type>,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let variant_name = self.bindings.get(binding).name().to_string();
        let site = self.variant_site(binding, expected);
        let labels = self
            .declarations
            .types
            .enums
            .variant_shape(site.info)
            .field_labels(fields)
            .ok_or_else(|| {
                TypeProblems::label_on_positional_payload(
                    &variant_name,
                    &fields[0].0,
                    site.info.arity,
                )
            })?;
        let declared: Vec<(Rc<str>, Type)> = labels
            .into_iter()
            .zip(site.payload.iter().cloned())
            .collect();
        let mut typed = Vec::new();
        let mut order = Vec::new();
        for (given, (_, value)) in fields.iter().enumerate() {
            let position = Self::field_position(&variant_name, &declared, fields, given)?;
            let field_type = &declared[position].1;
            typed.push(self.infer_against(value, field_type, MismatchContext::Plain)?);
            order.push(position);
        }
        Self::require_every_field(&variant_name, &declared, fields, span)?;
        Ok(TypedExpr {
            kind: TypedExprKind::Variant(binding, typed, FieldOrder::of(order)),
            ty: Type::Named(site.info.owner, site.type_arguments),
            span,
        })
    }
}
