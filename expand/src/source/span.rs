use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SourceId(u32);

impl SourceId {
    const PINNED: u32 = 1 << 31;

    pub fn new(index: usize) -> Self {
        let id = u32::try_from(index).expect("fewer than 2^31 source files");
        assert!(id < Self::PINNED, "fewer than 2^31 source files");
        Self(id)
    }

    pub fn pinned(self) -> Self {
        Self(self.0 | Self::PINNED)
    }

    pub fn is_pinned(self) -> bool {
        self.0 & Self::PINNED != 0
    }

    pub fn index(self) -> usize {
        (self.0 & !Self::PINNED) as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Span {
    source: SourceId,
    start: u32,
    end: u32,
}

impl Span {
    pub fn new(source: SourceId, range: Range<usize>) -> Self {
        Self {
            source,
            start: Self::offset(range.start),
            end: Self::offset(range.end),
        }
    }

    pub fn is_pinned(self) -> bool {
        self.source.is_pinned()
    }

    pub fn source(self) -> SourceId {
        self.source
    }

    pub fn range(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }

    pub fn start(self) -> Span {
        Span::new(self.source(), self.range().start..self.range().start)
    }

    pub fn end(self) -> Span {
        Span::new(self.source(), self.range().end..self.range().end)
    }

    pub fn contains(self, inner: Span) -> bool {
        self.source == inner.source && self.start <= inner.start && inner.end <= self.end
    }

    fn offset(position: usize) -> u32 {
        u32::try_from(position).expect("source files are smaller than 4 GiB")
    }
}
