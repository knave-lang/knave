use super::macro_origins::MacroOrigins;
use super::origin_row::OriginRow;
use super::saved_index::SavedWords;
use super::saved_origin_rows::SavedOriginRows;
use crate::source::Span;

#[derive(Default, serde::Serialize)]
pub struct OriginTables {
    rows: Vec<u8>,
    origins: Vec<u8>,
}

impl OriginTables {
    pub fn of_own_layers(origins: MacroOrigins) -> Self {
        let mut introduced = origins.introduced.into_own().into_names_by_call_site();
        let mut tables = Self::default();
        for (call_site, origin) in origins.calls.into_own().into_rows() {
            let introduced = introduced.remove(&call_site).unwrap_or_default();
            tables.add(call_site, &OriginRow { origin, introduced });
        }
        tables
    }

    fn add(&mut self, call_site: Span, row: &OriginRow) {
        let start = SavedWords::index(self.origins.len());
        let encoded = bincode::serialize(row).expect("every macro origin serializes");
        self.origins.extend(encoded);
        let [source, span_start, span_end] = SavedOriginRows::key_of(call_site);
        let end = SavedWords::index(self.origins.len());
        SavedWords::write(&[source, span_start, span_end, start, end], &mut self.rows);
    }
}
