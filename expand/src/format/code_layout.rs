use super::code_spacing::CodeSpacing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeLayout {
    pub column: usize,
    pub spacing: CodeSpacing,
}
