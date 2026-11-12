use std::ops::Range;

use super::macro_origin::MacroOrigin;
use super::origin_row::{IntroducedNames, OriginRow};
use super::saved_index::SavedWords;
use crate::source::{SourceId, Span};

#[derive(Clone, Copy, Default)]
pub struct SavedOrigins {
    rows: &'static [u8],
    origins: &'static [u8],
}

impl<'de: 'static> serde::Deserialize<'de> for SavedOrigins {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (rows, origins) = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self { rows, origins })
    }
}

impl SavedOrigins {
    pub fn origin_at(&self, call_site: Span) -> Option<MacroOrigin> {
        let row = self.row_at(call_site)?;
        Some(self.decode(row).0)
    }

    pub fn introduced(&self, call_site: Span, name: &str) -> Option<Span> {
        let (_, names) = self.decode(self.row_at(call_site)?);
        let found = names.iter().find(|(held, _)| held == name);
        found.map(|(_, span)| *span)
    }

    pub fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
        let containing = self
            .rows_of_source(span.source())
            .filter(|row| OriginRow::call_site(self.row(*row)).contains(span));
        let smallest =
            containing.min_by_key(|row| OriginRow::call_site(self.row(*row)).range().len());
        smallest.map(|row| self.decode(row).0)
    }

    fn count(&self) -> usize {
        self.rows.len() / OriginRow::SIZE
    }

    fn row(&self, index: usize) -> &'static [u8] {
        &self.rows[index * OriginRow::SIZE..(index + 1) * OriginRow::SIZE]
    }

    fn row_at(&self, call_site: Span) -> Option<usize> {
        let key = OriginRow::key_of(call_site);
        let index = self.first_row_at_or_after(key);
        (index < self.count() && OriginRow::key(self.row(index)) == key).then_some(index)
    }

    fn rows_of_source(&self, source: SourceId) -> Range<usize> {
        let word = SavedWords::index(source.index());
        self.first_row_at_or_after([word, 0, 0])..self.first_row_at_or_after([word + 1, 0, 0])
    }

    fn first_row_at_or_after(&self, key: [u32; OriginRow::KEY_WORDS]) -> usize {
        let (mut low, mut high) = (0, self.count());
        while low < high {
            let middle = low.midpoint(high);
            if OriginRow::key(self.row(middle)) < key {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }

    fn decode(&self, index: usize) -> (MacroOrigin, IntroducedNames) {
        let row = self.row(index);
        let start = OriginRow::word(row, OriginRow::START) as usize;
        let end = OriginRow::word(row, OriginRow::END) as usize;
        bincode::deserialize(&self.origins[start..end]).expect("a saved macro origin decodes")
    }
}
