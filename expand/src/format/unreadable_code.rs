use super::unreadable_cause::UnreadableCause;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnreadableCode {
    pub line: usize,
    pub cause: UnreadableCause,
}
