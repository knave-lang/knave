use super::unreadable_cause::UnreadableCause;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct FormatProblems;

impl FormatProblems {
    pub fn unreadable_docstring_code(line: Span, cause: UnreadableCause) -> Diagnostic {
        Diagnostic::warning(
            DiagnosticCode::UNREADABLE_DOCSTRING_CODE,
            "this docstring code does not read, so `fmt` leaves it as written",
            Label::new(line, "the code from this line on does not read as forms"),
        )
        .with_help(match cause {
            UnreadableCause::Code => {
                "close each list and string, or name the code's language after the fence's backticks, as in ```rust"
            }
            UnreadableCause::ShallowResult => {
                "indent the result's later lines deeper than the code, so they continue the result"
            }
        })
    }
}
