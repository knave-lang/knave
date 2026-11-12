use std::ops::Range;

use super::code_spacing::CodeSpacing;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeLines {
    pub range: Range<usize>,
    pub spacing: CodeSpacing,
}
