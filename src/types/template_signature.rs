use super::index_problems::IndexProblems;
use super::placeholder_site::PlaceholderSite;
use crate::core::{BindingId, Conversion, RustTemplate, TypeExpr, TypeExprKind};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::Span;

pub(super) struct TemplateSignature<'t> {
    parameters: &'t [TypeExpr],
    result: Option<&'t TypeExpr>,
    option: Option<BindingId>,
    span: Span,
}

impl<'t> TemplateSignature<'t> {
    pub(super) fn new(template: &'t RustTemplate, option: Option<BindingId>, span: Span) -> Self {
        let (parameters, result) = Self::function_parts(&template.type_expr);
        Self {
            parameters,
            result,
            option,
            span,
        }
    }

    pub(super) fn index_problems(&self, conversions: &[(usize, Conversion)]) -> Diagnostics {
        conversions
            .iter()
            .filter_map(|(position, conversion)| self.problem(*position, *conversion))
            .collect()
    }

    fn function_parts(written: &TypeExpr) -> (&[TypeExpr], Option<&TypeExpr>) {
        match written.kind() {
            TypeExprKind::Where(_, body) => Self::function_parts(body),
            TypeExprKind::Function(_, parameters, result) => (parameters, Some(result)),
            _ => (&[], None),
        }
    }

    fn problem(&self, position: usize, conversion: Conversion) -> Option<Diagnostic> {
        let Some(parameter) = self.parameters.get(position) else {
            let site = self.site(position, conversion, None);
            return Some(IndexProblems::missing_index_parameter(&site));
        };
        if !Self::is_converted(parameter) {
            let site = self.site(position, conversion, Some(parameter));
            return Some(IndexProblems::unconverted_index_parameter(&site));
        }
        (conversion.yields_option() && !self.returns_option()).then(|| {
            IndexProblems::optional_index_without_option(&self.site(
                position,
                conversion,
                self.result,
            ))
        })
    }

    fn is_converted(parameter: &TypeExpr) -> bool {
        matches!(parameter.kind(), TypeExprKind::Primitive(primitive) if *primitive == Conversion::ARGUMENT)
    }

    fn returns_option(&self) -> bool {
        self.result.is_some_and(|result| {
            matches!(result.kind(), TypeExprKind::Named(binding, _) if Some(*binding) == self.option)
        })
    }

    fn site(
        &self,
        position: usize,
        conversion: Conversion,
        written: Option<&TypeExpr>,
    ) -> PlaceholderSite {
        PlaceholderSite {
            position,
            conversion,
            span: written.map_or(self.span, TypeExpr::span),
        }
    }
}
