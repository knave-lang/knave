use crate::core::{PassingAspect, PassingDifference};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct ClausePassingProblems;

impl ClausePassingProblems {
    const SAME_PASSING_HELP: &'static str = "write each placeholder as the template writes it, with the same borrow, receiver, index or call";

    pub(super) fn arguments_unlike_template(
        difference: PassingDifference,
        clause: Span,
        template: Span,
    ) -> Diagnostic {
        let (clause_label, template_label, help) = match difference {
            PassingDifference::Argument(argument, aspect) => (
                Self::passing_aspect_label(argument, aspect),
                format!("unlike the template's `{{{argument}}}` here"),
                Self::SAME_PASSING_HELP,
            ),
            PassingDifference::CallOnlyInClause => (
                "this clause calls a function".to_string(),
                "the template calls no function".to_string(),
                Self::SAME_PASSING_HELP,
            ),
            PassingDifference::ImpureClause => (
                "this clause differs from the template but is not a plain expression".to_string(),
                "the template passes its arguments differently".to_string(),
                "differ only in an expression of names, literals, path calls and operators besides `=`, `&` and `|`",
            ),
        };
        Diagnostic::error(
            DiagnosticCode::CLAUSE_PASSES_ARGUMENTS_UNLIKE_TEMPLATE,
            "a `rust` clause must pass its arguments as its template does",
            Label::new(clause, clause_label),
        )
        .with_label(Label::new(template, template_label))
        .with_help(help)
    }

    fn passing_aspect_label(argument: usize, aspect: PassingAspect) -> String {
        let placeholder = format!("`{{{argument}}}`");
        match aspect {
            PassingAspect::Use => format!("this clause differs in whether it uses {placeholder}"),
            PassingAspect::Borrow => format!("this clause borrows {placeholder} differently"),
            PassingAspect::Receiver => {
                format!("this clause differs in whether {placeholder} receives a method call")
            }
            PassingAspect::Index => {
                format!("this clause differs in whether it indexes {placeholder}")
            }
            PassingAspect::Dereference => {
                format!("this clause differs in whether it dereferences {placeholder}")
            }
            PassingAspect::Call => format!("this clause differs in whether it calls {placeholder}"),
            PassingAspect::Operand => {
                format!("this clause differs in whether {placeholder} is an operator's operand")
            }
        }
    }
}
