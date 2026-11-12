use std::ops::Range;

use super::format_problems::FormatProblems;
use super::unreadable_cause::UnreadableCause;
use crate::diagnostic::Diagnostic;
use crate::source::{SourceId, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnreadableSpan {
    pub range: Range<usize>,
    pub cause: UnreadableCause,
}

impl UnreadableSpan {
    pub fn after(&self, start: usize) -> Self {
        Self {
            range: start + self.range.start..start + self.range.end,
            cause: self.cause,
        }
    }

    pub fn warning_in(&self, source: SourceId) -> Diagnostic {
        FormatProblems::unreadable_docstring_code(Span::new(source, self.range.clone()), self.cause)
    }
}

#[cfg(test)]
mod tests {
    use super::{UnreadableCause, UnreadableSpan};

    #[test]
    fn a_span_after_a_start_moves_both_ends() {
        let span = UnreadableSpan {
            range: 2..5,
            cause: UnreadableCause::Code,
        };
        assert_eq!(span.after(10).range, 12..15);
    }
}
