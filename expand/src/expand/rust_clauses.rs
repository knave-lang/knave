use super::Expander;
use super::clause_chooser_problems::ClauseChooserProblems;
use super::clause_passing_problems::ClausePassingProblems;
use super::conversion_problems::ConversionProblems;
use super::copy_implementors::CopyImplementors;
use super::problems::ExpandProblems;
use super::shape::Shape;
use crate::core::{
    ArgumentPositions, ClauseComparison, ParsedTemplate, PassingAllowance, Piece, ReferenceKind,
    RustClause, TypeBinder, TypeExpr, TypeExprKind,
};
use crate::diagnostic::Diagnostic;
use crate::reader::Form;
use crate::source::Span;

pub(super) struct ClausedTemplate<'t> {
    parsed: &'t ParsedTemplate,
    span: Span,
    type_expr: &'t TypeExpr,
    type_span: Span,
    shared_references: ArgumentPositions,
    chooser_arguments: ArgumentPositions,
}

impl<'t> ClausedTemplate<'t> {
    pub(super) fn of(parsed: &'t ParsedTemplate, type_expr: &'t TypeExpr, items: &[Form]) -> Self {
        Self {
            parsed,
            span: items[1].span(),
            type_expr,
            type_span: items[2].span(),
            shared_references: Self::parameter_positions(type_expr, Self::is_shared_reference),
            chooser_arguments: Self::chooser_arguments(type_expr),
        }
    }

    fn chooser_problem(&self) -> Option<Diagnostic> {
        let Some(chooser) = Self::chooser_of(self.type_expr) else {
            return Some(ClauseChooserProblems::missing(self.type_span));
        };
        let fixed = self
            .signature_types()
            .any(|written| written.named_bindings().contains(&chooser.variable));
        (!fixed).then(|| ClauseChooserProblems::unfixed(chooser.span))
    }

    fn signature_types(&self) -> impl Iterator<Item = &TypeExpr> {
        let body = match self.type_expr.kind() {
            TypeExprKind::Where(_, body) => Some(&**body),
            _ => None,
        };
        let (parameters, result) = match body.map(TypeExpr::kind) {
            Some(TypeExprKind::Function(_, parameters, result)) => {
                (parameters.as_slice(), Some(&**result))
            }
            _ => (&[][..], body),
        };
        parameters.iter().chain(result)
    }

    fn passing_allowance(&self, target: &TypeExpr, copy: &CopyImplementors) -> PassingAllowance {
        let copied = if copy.lists_primitive(target) {
            self.chooser_arguments.clone()
        } else {
            ArgumentPositions::default()
        };
        PassingAllowance::new(self.shared_references.clone(), copied)
    }

    fn chooser_of(type_expr: &TypeExpr) -> Option<&TypeBinder> {
        Expander::where_binders(type_expr)
            .iter()
            .find(|binder| !binder.lifetime)
    }

    fn chooser_arguments(type_expr: &TypeExpr) -> ArgumentPositions {
        Self::chooser_of(type_expr).map_or_else(ArgumentPositions::default, |chooser| {
            Self::parameter_positions(type_expr, |parameter| {
                matches!(parameter.kind(), TypeExprKind::Named(variable, _)
                    if *variable == chooser.variable)
            })
        })
    }

    fn is_shared_reference(parameter: &TypeExpr) -> bool {
        matches!(
            parameter.kind(),
            TypeExprKind::Reference(ReferenceKind::Shared, _, _)
        )
    }

    fn parameter_positions(
        type_expr: &TypeExpr,
        selects: impl Fn(&TypeExpr) -> bool,
    ) -> ArgumentPositions {
        match type_expr.where_body().kind() {
            TypeExprKind::Function(_, parameters, _) => ArgumentPositions::new(
                parameters
                    .iter()
                    .enumerate()
                    .filter(|(_, parameter)| selects(parameter))
                    .map(|(position, _)| position)
                    .collect(),
            ),
            _ => ArgumentPositions::default(),
        }
    }
}

struct ClauseLowering<'c, 't> {
    template: &'c ClausedTemplate<'t>,
    copy: CopyImplementors,
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
        if pairs.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(problem) = template.chooser_problem() {
            return Err(problem);
        }
        let lowering = ClauseLowering {
            template,
            copy: CopyImplementors::in_modules(self.tables.expanded.values()),
        };
        pairs
            .into_iter()
            .map(|pair| self.lower_rust_clause(shape, &lowering, pair))
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
        lowering: &ClauseLowering,
        [target, text]: [&Form; 2],
    ) -> Result<RustClause, Diagnostic> {
        let template = lowering.template;
        let parsed = Self::parse_rust_template(shape.string(text)?, text.span())?;
        if Piece::conversions_in(parsed.pieces()).next().is_some() {
            return Err(ConversionProblems::in_clause_template(text.span()));
        }
        self.check_template_type_placeholders(&parsed, template.type_expr, text.span())?;
        let target = self.lower_type(target)?;
        let allowance = template.passing_allowance(&target, &lowering.copy);
        let dropped_borrows = ClauseComparison::new(template.parsed, &parsed, &allowance)
            .dropped_borrows()
            .map_err(|difference| {
                ClausePassingProblems::arguments_unlike_template(
                    difference,
                    text.span(),
                    template.span,
                )
            })?;
        Ok(RustClause {
            target,
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

    pub(super) fn check_template_type_placeholders(
        &self,
        template: &ParsedTemplate,
        type_expr: &TypeExpr,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let parameters: Vec<String> = Self::where_binders(type_expr)
            .iter()
            .map(|binder| self.tables.bindings.get(binder.variable).name().to_string())
            .collect();
        for name in template.type_placeholders() {
            if !parameters.iter().any(|parameter| parameter == name) {
                return Err(ExpandProblems::unknown_type_placeholder(
                    name,
                    &parameters,
                    span,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn where_binders(type_expr: &TypeExpr) -> &[TypeBinder] {
        match type_expr.kind() {
            TypeExprKind::Where(binders, _) => binders.as_slice(),
            _ => &[],
        }
    }
}
