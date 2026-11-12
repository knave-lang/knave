#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnreadableCause {
    Code,
    ShallowResult,
}
