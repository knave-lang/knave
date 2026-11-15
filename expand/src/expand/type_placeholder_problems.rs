use super::problems::ParameterExample;
use crate::diagnostic::{Counted, Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct TypePlaceholderProblems {
    index: usize,
    parameters: usize,
    span: Span,
}

impl TypePlaceholderProblems {
    pub fn new(index: usize, parameters: usize, span: Span) -> Self {
        Self {
            index,
            parameters,
            span,
        }
    }

    pub fn declared_on_name(&self, example: &ParameterExample<'_>) -> Diagnostic {
        self.without_parameter().with_help(format!(
            "declare the type parameters on the name, as in `{}`",
            example.definition()
        ))
    }

    pub fn in_clause(&self) -> Diagnostic {
        self.without_parameter()
            .with_help("write the placeholder as `{0}`")
    }

    fn without_parameter(&self) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::PLACEHOLDER_WITHOUT_PARAMETER,
            format!("`{{{}}}` names no type parameter", self.index),
            Label::new(
                self.span,
                format!(
                    "this type has {}",
                    Counted::noun(self.parameters, "type parameter")
                ),
            ),
        )
    }
}
