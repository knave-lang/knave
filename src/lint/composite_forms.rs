use super::form_reader::FormReader;
use super::spanned_forms::SpannedForms;
use crate::core::{
    BindingId, Expr, ExprKind, ExpressionForm, FieldLabel, KernelForm, StructExpr, StructForm,
};
use crate::reader::Form;
use crate::source::Span;

pub struct CompositeForms;

type KeyedUpdate<'e> = (ExpressionForm, &'e Expr, &'e [Expr], &'e Expr);

impl CompositeForms {
    const UPDATED_STRUCT: &'static str = "_";

    pub fn read(forms: &mut dyn FormReader<'_>, kind: &ExprKind, span: Span) -> Form {
        match kind {
            ExprKind::Call(callee, arguments) => Self::call(forms, (callee, arguments), span),
            ExprKind::Struct(struct_expr) => Self::struct_expr(forms, struct_expr, span),
            ExprKind::Variant(variant, arguments) => {
                Self::variant(forms, (*variant, arguments), span)
            }
            ExprKind::LabeledVariant(variant, fields) => {
                Self::labeled(forms, (*variant, fields), span)
            }
            ExprKind::Assoc(value, keys, item) => {
                Self::keyed(forms, (ExpressionForm::Assoc, value, keys, item), span)
            }
            ExprKind::SetAt(value, keys, item) => {
                Self::keyed(forms, (ExpressionForm::SetAt, value, keys, item), span)
            }
            other => Self::map_at(forms, other, span),
        }
    }

    fn map_at(forms: &mut dyn FormReader<'_>, kind: &ExprKind, span: Span) -> Form {
        let ExprKind::MapAt(operation, value, arguments) = kind else {
            unreachable!("every other expression kind has its own reader")
        };
        let row = forms.writer().open();
        let mut parts = vec![
            forms.kernel_head(KernelForm::Expression(ExpressionForm::MapAt), span),
            forms
                .writer()
                .leaf(SpannedForms::symbol(operation.keyword(), span)),
            forms.expr(value),
        ];
        parts.extend(arguments.iter().map(|argument| forms.expr(argument)));
        forms.writer().close(row, parts, span)
    }

    fn call(forms: &mut dyn FormReader<'_>, call: (&Expr, &[Expr]), span: Span) -> Form {
        let (callee, arguments) = call;
        let row = forms.writer().open();
        let mut parts = vec![forms.expr(callee)];
        parts.extend(arguments.iter().map(|argument| forms.expr(argument)));
        forms.writer().close(row, parts, span)
    }

    fn variant(forms: &mut dyn FormReader<'_>, variant: (BindingId, &[Expr]), span: Span) -> Form {
        let (binding, arguments) = variant;
        let row = forms.writer().open();
        let mut parts = vec![forms.writer().reference(binding, None, span)];
        parts.extend(arguments.iter().map(|argument| forms.expr(argument)));
        forms.writer().close(row, parts, span)
    }

    fn labeled(
        forms: &mut dyn FormReader<'_>,
        variant: (BindingId, &[(FieldLabel, Expr)]),
        span: Span,
    ) -> Form {
        let (binding, fields) = variant;
        let row = forms.writer().open();
        let mut parts = vec![forms.writer().reference(binding, None, span)];
        parts.extend(Self::fields(forms, fields));
        forms.writer().close(row, parts, span)
    }

    fn struct_expr(forms: &mut dyn FormReader<'_>, struct_expr: &StructExpr, span: Span) -> Form {
        let row = forms.writer().open();
        let parts = match struct_expr {
            StructExpr::New(name, fields) => {
                let mut parts = vec![
                    Self::struct_head(forms, StructForm::New, span),
                    forms.writer().reference(*name, None, span),
                ];
                parts.extend(Self::fields(forms, fields));
                parts
            }
            StructExpr::Update(base, fields) => {
                let mut parts = vec![
                    Self::struct_head(forms, StructForm::New, span),
                    forms
                        .writer()
                        .leaf(SpannedForms::symbol(Self::UPDATED_STRUCT, span)),
                    forms.expr(base),
                ];
                parts.extend(Self::fields(forms, fields));
                parts
            }
            StructExpr::Field(value, label) => vec![
                Self::struct_head(forms, StructForm::Field, span),
                forms.expr(value),
                forms
                    .writer()
                    .leaf(SpannedForms::symbol(&label.name, label.span)),
            ],
        };
        forms.writer().close(row, parts, span)
    }

    fn struct_head(forms: &mut dyn FormReader<'_>, form: StructForm, span: Span) -> Form {
        forms.kernel_head(KernelForm::Expression(ExpressionForm::Struct(form)), span)
    }

    fn fields(forms: &mut dyn FormReader<'_>, fields: &[(FieldLabel, Expr)]) -> Vec<Form> {
        fields
            .iter()
            .map(|(label, value)| {
                let row = forms.writer().open();
                let parts = vec![
                    forms
                        .writer()
                        .leaf(SpannedForms::symbol(&label.name, label.span)),
                    forms.expr(value),
                ];
                forms.writer().close(row, parts, label.span)
            })
            .collect()
    }

    fn keyed(forms: &mut dyn FormReader<'_>, update: KeyedUpdate<'_>, span: Span) -> Form {
        let (form, value, keys, item) = update;
        let row = forms.writer().open();
        let head = forms.kernel_head(KernelForm::Expression(form), span);
        let value = forms.expr(value);
        let keys_row = forms.writer().open();
        let keys = keys.iter().map(|key| forms.expr(key)).collect();
        let keys = forms.writer().close(keys_row, keys, span);
        let parts = vec![head, value, keys, forms.expr(item)];
        forms.writer().close(row, parts, span)
    }
}
