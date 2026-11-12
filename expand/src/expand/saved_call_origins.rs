use super::macro_origin::MacroOrigin;
use super::saved_origins::SavedOrigins;
use crate::source::Span;

#[derive(Default)]
pub struct SavedCallOrigins {
    saved: SavedOrigins,
}

impl SavedCallOrigins {
    pub fn new(saved: SavedOrigins) -> Self {
        Self { saved }
    }

    pub fn at(&self, call_site: Span) -> Option<MacroOrigin> {
        self.saved.origin_at(call_site)
    }

    pub fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
        self.saved.enclosing(span)
    }
}

#[cfg(test)]
mod tests {
    use crate::expand::origins_fixture::OriginsFixture;

    #[test]
    fn a_saved_origin_is_found_by_its_exact_call_site() {
        let found = OriginsFixture::saved_calls().at(OriginsFixture::call_site());
        assert_eq!(OriginsFixture::name(found), Some("outer".to_string()));
    }

    #[test]
    fn a_span_that_is_no_call_site_has_no_saved_origin() {
        let saved = OriginsFixture::saved_calls();
        assert!(saved.at(OriginsFixture::span(1, 11, 19)).is_none());
    }

    #[test]
    fn a_call_site_past_the_last_saved_one_has_no_origin() {
        let saved = OriginsFixture::saved_calls();
        assert!(saved.at(OriginsFixture::span(9, 0, 1)).is_none());
    }

    #[test]
    fn the_smallest_saved_call_site_around_a_span_encloses_it() {
        let found = OriginsFixture::saved_calls().enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("inner".to_string()));
    }

    #[test]
    fn a_span_outside_every_saved_call_site_is_not_enclosed() {
        let saved = OriginsFixture::saved_calls();
        assert!(saved.enclosing(OriginsFixture::span(1, 30, 31)).is_none());
    }

    #[test]
    fn a_saved_call_site_in_another_source_does_not_enclose() {
        let saved = OriginsFixture::saved_calls();
        assert!(saved.enclosing(OriginsFixture::span(3, 13, 14)).is_none());
    }
}
