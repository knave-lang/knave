use super::builtin_site::BuiltinSite;
use super::step_budget::StepBudget;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct LimitProblems;

impl LimitProblems {
    pub const BUILT_LIST_NESTS_TOO_DEEP: &'static str = "macro code built a list nested too deeply";
    const VALUE_TOO_LARGE: &'static str = "macro code built a value too large";
    const RAISED_LIMIT_FACTOR: u64 = 10;

    pub fn too_many_steps(limit: u64, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MACRO_STEP_LIMIT,
            "macro code ran too many steps",
            Label::new(span, format!("this call passed {limit} macro steps")),
        )
        .with_note(format!(
            "the macro calls in one top-level form run at most {limit} steps: a function call or a `recur` costs one step per expression of the body it runs, an `expand-call` costs {} and one per form it returns, a closure costs one step and one per binding in scope, and a built-in costs one step per list item it copies or reads and one for every {} bytes of text it reads, and `closest-name` one step for each pair of bytes it compares between the name and the candidates",
            StepBudget::EXPAND_CALL_WORK,
            StepBudget::BYTES_PER_STEP
        ))
        .with_help(format!(
            "make the macro stop, or wrap the top-level form in `(with-macro-steps {} ...)`",
            limit
                .saturating_mul(Self::RAISED_LIMIT_FACTOR)
                .max(StepBudget::DEFAULT_LIMIT)
        ))
    }

    pub fn list_too_large(limit: usize, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::EXPANSION_TOO_LARGE,
            "macro code used a list too large to read as code",
            Label::new(span, format!("this call reads a list past {limit} forms")),
        )
        .with_note(format!(
            "a list that macro code passes to a built-in reads as at most {limit} forms, counting each shared part once per use"
        ))
        .with_help("build a smaller list, or stop repeating the same list inside itself")
    }

    pub fn value_nests_too_deep(limit: usize, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::EXPANSION_NESTS_TOO_DEEP,
            Self::BUILT_LIST_NESTS_TOO_DEEP,
            Label::new(span, format!("this call builds a list past {limit} levels")),
        )
        .with_note(format!(
            "lists that macro code builds nest at most {limit} deep"
        ))
        .with_help("make the macro stop nesting, or build a flatter list")
    }

    pub fn text_too_long(site: BuiltinSite, bytes: usize, limit: usize) -> Diagnostic {
        let label = format!("`{}` builds text of {bytes} bytes here", site.function);
        Self::value_too_large(label, site.span)
            .with_note(format!(
                "text that macro code builds holds at most {limit} bytes"
            ))
            .with_help("build smaller text, or make the macro stop growing it")
    }

    pub fn built_list_too_large(site: BuiltinSite, size: usize, limit: usize) -> Diagnostic {
        let label = format!("`{}` builds a list of {size} values here", site.function);
        Self::value_too_large(label, site.span)
            .with_note(format!(
                "a list that macro code builds holds at most {limit} values, counting each value inside it"
            ))
            .with_help("build a smaller list, or make the macro stop growing it")
    }

    pub fn too_many_module_notes(site: BuiltinSite, notes: usize, limit: usize) -> Diagnostic {
        let label = format!("`{}` adds module note {notes} here", site.function);
        Self::value_too_large(label, site.span)
            .with_note(format!("a program holds at most {limit} module notes"))
            .with_help("add fewer module notes")
    }

    fn value_too_large(label: String, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MACRO_VALUE_SIZE,
            Self::VALUE_TOO_LARGE,
            Label::new(span, label),
        )
    }
}
