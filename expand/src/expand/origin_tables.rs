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

#[cfg(test)]
mod tests {
    use crate::expand::introduced_names::IntroducedNames;
    use crate::expand::macro_origins::MacroOrigins;
    use crate::expand::origins_fixture::OriginsFixture;
    use crate::expand::saved_introductions::SavedIntroductions;

    #[test]
    fn a_saved_row_keeps_the_names_introduced_at_its_call_site() {
        let rows = OriginsFixture::saved_rows();
        let row = rows.decode(
            rows.row_at(OriginsFixture::call_site())
                .expect("a saved row"),
        );
        assert_eq!(row.origin.name.as_ref(), "outer");
        let names: Vec<&str> = row
            .introduced
            .iter()
            .map(|held| held.name.as_str())
            .collect();
        assert_eq!(names, ["helper"]);
    }

    #[test]
    fn a_name_introduced_where_no_macro_was_called_is_not_saved() {
        let mut origins = MacroOrigins::default();
        let call_site = OriginsFixture::call_site();
        origins
            .introduced
            .record(call_site, "x", OriginsFixture::span(1, 1, 2));
        let saved = SavedIntroductions::new(OriginsFixture::saved_rows_of(origins));
        let introduced = IntroducedNames::over(saved);
        assert!(introduced.at(call_site, "x").is_none());
    }
}
