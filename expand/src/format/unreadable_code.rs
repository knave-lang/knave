use std::ops::Range;

use super::unreadable_cause::UnreadableCause;
use super::unreadable_span::UnreadableSpan;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnreadableCode {
    pub line: usize,
    pub cause: UnreadableCause,
}

impl UnreadableCode {
    pub fn spanning(self, range: Range<usize>) -> UnreadableSpan {
        UnreadableSpan::new(range, self.cause)
    }
}
