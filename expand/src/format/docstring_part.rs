use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocstringPart {
    Prose(Range<usize>),
    Example(Range<usize>),
    Fence(Range<usize>),
}

impl DocstringPart {
    pub fn range(&self) -> Range<usize> {
        match self {
            Self::Prose(range) | Self::Example(range) | Self::Fence(range) => range.clone(),
        }
    }
}
