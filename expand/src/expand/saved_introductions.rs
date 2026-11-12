use super::saved_origin_rows::SavedOriginRows;
use crate::source::Span;

#[derive(Default)]
pub struct SavedIntroductions {
    rows: SavedOriginRows,
}

impl SavedIntroductions {
    pub fn new(rows: SavedOriginRows) -> Self {
        Self { rows }
    }

    pub fn at(&self, call_site: Span, name: &str) -> Option<Span> {
        let row = self.rows.decode(self.rows.row_at(call_site)?);
        let found = row.introduced.into_iter().find(|held| held.name == name);
        found.map(|held| held.written)
    }
}
