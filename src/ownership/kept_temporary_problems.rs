use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;
use crate::types::Type;

#[derive(Clone, Copy)]
pub enum KeptBorrow {
    OfParameter,
    HeldByParameter,
}

impl KeptBorrow {
    pub fn of(parameter: &Type) -> Self {
        if parameter.is_reference() {
            Self::OfParameter
        } else {
            Self::HeldByParameter
        }
    }
}

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
    pub kept_borrow: KeptBorrow,
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
            kept_borrow,
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
            Self::kept_borrow_label(*kept_borrow, parameter),
        ))
        .with_help(wording.help)
    }

    fn kept_borrow_label(kept: KeptBorrow, parameter: &str) -> String {
        match kept {
            KeptBorrow::OfParameter => {
                format!("the returned closure keeps this borrow of `{parameter}`")
            }
            KeptBorrow::HeldByParameter => {
                format!("the returned closure keeps the borrow that `{parameter}` holds")
            }
        }
    }

    fn wording(kept_until: KeptUntil) -> KeptWording {
        match kept_until {
            KeptUntil::StatementEnd => KeptWording {
                dropped: "this temporary is dropped at the end of the statement",
                help: "bind the argument with `let` before the statement that binds the result, so it lives as long as the result",
            },
            KeptUntil::FunctionReturn => KeptWording {
                dropped: "this temporary is dropped when the function returns, while the result still borrows it",
                help: "take the value as a parameter of the function that returns this result, so the caller keeps it alive",
            },
        }
    }
}
