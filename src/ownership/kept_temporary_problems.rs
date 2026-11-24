use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

#[derive(Clone, Copy)]
pub enum KeptUntil {
    StatementEnd,
    FunctionReturn,
}

pub struct KeptTemporary<'n> {
    pub argument: Span,
    pub callee: &'n str,
    pub parameter: &'n str,
    pub parameter_span: Span,
    pub kept_until: KeptUntil,
}

struct KeptWording {
    dropped: &'static str,
    help: &'static str,
}

pub struct KeptTemporaryProblems;

impl KeptTemporaryProblems {
    pub fn kept_temporary(kept: &KeptTemporary<'_>) -> Diagnostic {
        let KeptTemporary {
            argument,
            callee,
            parameter,
            parameter_span,
            kept_until,
        } = kept;
        let wording = Self::wording(*kept_until);
        Diagnostic::error(
            DiagnosticCode::KEPT_TEMPORARY_ARGUMENT,
            format!("the result of `{callee}` keeps a borrow of a temporary"),
            Label::new(*argument, wording.dropped),
        )
        .with_label(Label::new(
            *parameter_span,
            format!("the returned closure keeps this borrow of `{parameter}`"),
        ))
        .with_help(wording.help)
    }

    fn wording(kept_until: KeptUntil) -> KeptWording {
        match kept_until {
            KeptUntil::StatementEnd => KeptWording {
                dropped: "this temporary is dropped at the end of the statement",
                help: "bind the argument with `let` before the call, so it lives as long as the result",
            },
            KeptUntil::FunctionReturn => KeptWording {
                dropped: "this temporary is dropped when the function returns, while the result still borrows it",
                help: "take the value as a parameter of the function that returns this result, so the caller keeps it alive",
            },
        }
    }
}
