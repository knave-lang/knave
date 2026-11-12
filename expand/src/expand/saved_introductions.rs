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

#[cfg(test)]
mod tests {
    use crate::expand::origins_fixture::OriginsFixture;

    #[test]
    fn a_saved_name_is_found_where_it_was_written() {
        let saved = OriginsFixture::saved_introductions();
        let found = saved.at(OriginsFixture::call_site(), "helper");
        assert_eq!(found, Some(OriginsFixture::span(0, 5, 6)));
    }

    #[test]
    fn another_name_at_a_saved_call_site_is_not_found() {
        let saved = OriginsFixture::saved_introductions();
        assert!(saved.at(OriginsFixture::call_site(), "other").is_none());
    }

    #[test]
    fn a_name_at_a_saved_call_site_that_introduced_none_is_not_found() {
        let saved = OriginsFixture::saved_introductions();
        assert!(
            saved
                .at(OriginsFixture::span(1, 12, 15), "helper")
                .is_none()
        );
    }
}
