use std::ops::Range;

use super::origin_row::OriginRow;
use super::saved_index::SavedWords;
use crate::source::{SourceId, Span};

#[derive(Clone, Copy, Default)]
pub struct SavedOriginRows {
    rows: &'static [u8],
    origins: &'static [u8],
}

impl<'de: 'static> serde::Deserialize<'de> for SavedOriginRows {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (rows, origins) = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self { rows, origins })
    }
}

impl SavedOriginRows {
    const WORDS: usize = 5;
    const SIZE: usize = Self::WORDS * SavedWords::SIZE;
    const KEY_WORDS: usize = 3;
    const START: usize = 3;
    const END: usize = 4;

    pub fn key_of(call_site: Span) -> [u32; Self::KEY_WORDS] {
        let range = call_site.range();
        [
            SavedWords::index(call_site.source().index()),
            SavedWords::index(range.start),
            SavedWords::index(range.end),
        ]
    }

    pub fn row_at(&self, call_site: Span) -> Option<usize> {
        let key = Self::key_of(call_site);
        let index = self.first_row_at_or_after(key);
        (index < self.count() && Self::key(self.row(index)) == key).then_some(index)
    }

    pub fn rows_of_source(&self, source: SourceId) -> Range<usize> {
        let word = SavedWords::index(source.index());
        self.first_row_at_or_after([word, 0, 0])..self.first_row_at_or_after([word + 1, 0, 0])
    }

    pub fn call_site(&self, index: usize) -> Span {
        let [source, start, end] = Self::key(self.row(index));
        Span::new(SourceId::new(source as usize), start as usize..end as usize)
    }

    pub fn decode(&self, index: usize) -> OriginRow {
        let row = self.row(index);
        let start = Self::word(row, Self::START) as usize;
        let end = Self::word(row, Self::END) as usize;
        bincode::deserialize(&self.origins[start..end]).expect("a saved macro origin decodes")
    }

    fn count(&self) -> usize {
        self.rows.len() / Self::SIZE
    }

    fn row(&self, index: usize) -> &'static [u8] {
        &self.rows[index * Self::SIZE..(index + 1) * Self::SIZE]
    }

    fn first_row_at_or_after(&self, key: [u32; Self::KEY_WORDS]) -> usize {
        let (mut low, mut high) = (0, self.count());
        while low < high {
            let middle = low.midpoint(high);
            if Self::key(self.row(middle)) < key {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }

    fn word(row: &[u8], field: usize) -> u32 {
        SavedWords::read(&row[field * SavedWords::SIZE..])
    }

    fn key(row: &[u8]) -> [u32; Self::KEY_WORDS] {
        [Self::word(row, 0), Self::word(row, 1), Self::word(row, 2)]
    }
}

#[cfg(test)]
mod tests {
    use crate::expand::origins_fixture::OriginsFixture;
    use crate::source::SourceId;

    #[test]
    fn the_rows_of_a_source_hold_only_its_call_sites() {
        let rows = OriginsFixture::saved_rows();
        let call_sites: Vec<_> = rows
            .rows_of_source(SourceId::new(1))
            .map(|row| rows.call_site(row))
            .collect();
        assert_eq!(
            call_sites,
            [OriginsFixture::call_site(), OriginsFixture::span(1, 12, 15)]
        );
    }

    #[test]
    fn a_span_that_is_no_saved_call_site_has_no_row() {
        let rows = OriginsFixture::saved_rows();
        assert!(rows.row_at(OriginsFixture::span(1, 10, 21)).is_none());
    }

    #[test]
    fn a_row_decodes_to_the_origin_saved_at_its_call_site() {
        let rows = OriginsFixture::saved_rows();
        let row = rows
            .row_at(OriginsFixture::span(2, 10, 20))
            .expect("a saved row");
        assert_eq!(rows.decode(row).origin.name.as_ref(), "elsewhere");
    }
}
