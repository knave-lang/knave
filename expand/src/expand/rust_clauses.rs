use super::Expander;
use super::clause_chooser_problems::ClauseChooserProblems;
use super::problems::ExpandProblems;
use super::shape::Shape;
use crate::core::{
    ParsedTemplate, RustClause, SharedReferences, TypeBinder, TypeExpr, TypeExprKind,
};
use crate::diagnostic::Diagnostic;
use crate::reader::Form;
use crate::source::Span;

pub(super) struct ClausedTemplate<'t> {
    parsed: &'t ParsedTemplate,
    span: Span,
    type_expr: &'t TypeExpr,
    type_span: Span,
    shared_references: SharedReferences,
}

impl<'t> ClausedTemplate<'t> {
    pub(super) fn of(parsed: &'t ParsedTemplate, type_expr: &'t TypeExpr, items: &[Form]) -> Self {
        Self {
            parsed,
            span: items[1].span(),
            type_expr,
            type_span: items[2].span(),
            shared_references: type_expr.shared_reference_parameters(),
        }
    }

    fn chooser_problem(&self) -> Option<Diagnostic> {
        let Some(chooser) = Expander::where_binders(self.type_expr)
            .iter()
            .find(|binder| !binder.lifetime)
        else {
            return Some(ClauseChooserProblems::missing(self.type_span));
        };
        let fixed = self
            .argument_types()
            .iter()
            .any(|argument| argument.named_bindings().contains(&chooser.variable));
        (!fixed).then(|| ClauseChooserProblems::fixed_by_no_argument(chooser.span))
    }

    fn argument_types(&self) -> &[TypeExpr] {
        let TypeExprKind::Where(_, body) = self.type_expr.kind() else {
            return &[];
        };
        match body.kind() {
            TypeExprKind::Function(_, parameters, _) => parameters,
            _ => &[],
        }
    }
}

impl Expander<'_> {
    pub(super) const RUST_PARTS: usize = 3;

    pub(super) fn lower_rust_clauses(
        &mut self,
        shape: &Shape,
        template: &ClausedTemplate,
        clauses: &[Form],
    ) -> Result<Vec<RustClause>, Diagnostic> {
        let pairs = clauses
            .iter()
            .map(|clause| Self::clause_pair(shape, clause))
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        if !pairs.is_empty()
            && let Some(problem) = template.chooser_problem()
        {
            return Err(problem);
        }
        pairs
            .into_iter()
            .map(|pair| self.lower_rust_clause(shape, template, pair))
            .collect()
    }

    fn clause_pair<'f>(shape: &Shape, clause: &'f Form) -> Result<[&'f Form; 2], Diagnostic> {
        match clause.as_list() {
            Some([target, text]) => Ok([target, text]),
            _ => Err(shape.malformed(clause)),
        }
    }

    fn lower_rust_clause(
        &mut self,
        shape: &Shape,
        template: &ClausedTemplate,
        [target, text]: [&Form; 2],
    ) -> Result<RustClause, Diagnostic> {
        let parsed = Self::parse_rust_template(shape.string(text)?, text.span())?;
        self.check_template_type_placeholders(&parsed, template.type_expr, text.span())?;
        let dropped_borrows = template
            .parsed
            .clause_borrows(&parsed, &template.shared_references)
            .map_err(|difference| {
                ExpandProblems::clause_passes_arguments_unlike_template(
                    difference,
                    text.span(),
                    template.span,
                )
            })?;
        Ok(RustClause {
            target: self.lower_type(target)?,
            template: parsed.text().into(),
            dropped_borrows,
        })
    }

    pub(super) fn parse_rust_template(
        text: &str,
        span: Span,
    ) -> Result<ParsedTemplate, Diagnostic> {
        ParsedTemplate::parse(text).map_err(|_| ExpandProblems::malformed_template(span))
    }

    pub(super) fn where_binders(type_expr: &TypeExpr) -> &[TypeBinder] {
        match type_expr.kind() {
            TypeExprKind::Where(binders, _) => binders.as_slice(),
            _ => &[],
        }
    }
}
