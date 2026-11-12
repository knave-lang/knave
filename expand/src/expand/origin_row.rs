use super::saved_index::SavedWords;
use crate::source::{SourceId, Span};

pub type IntroducedNames = Vec<(String, Span)>;

pub struct OriginRow;

impl OriginRow {
    const WORDS: usize = 5;
    pub const SIZE: usize = Self::WORDS * SavedWords::SIZE;
    pub const KEY_WORDS: usize = 3;
    pub const START: usize = 3;
    pub const END: usize = 4;

    pub fn key_of(span: Span) -> [u32; Self::KEY_WORDS] {
        let range = span.range();
        [
            SavedWords::index(span.source().index()),
            SavedWords::index(range.start),
            SavedWords::index(range.end),
        ]
    }

    pub fn word(row: &[u8], field: usize) -> u32 {
        SavedWords::read(&row[field * SavedWords::SIZE..])
    }

    pub fn key(row: &[u8]) -> [u32; Self::KEY_WORDS] {
        [Self::word(row, 0), Self::word(row, 1), Self::word(row, 2)]
    }

    pub fn call_site(row: &[u8]) -> Span {
        let [source, start, end] = Self::key(row);
        Span::new(SourceId::new(source as usize), start as usize..end as usize)
    }
}
