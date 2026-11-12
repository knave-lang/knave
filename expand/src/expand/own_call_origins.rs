use std::cell::OnceCell;

use rustc_hash::FxHashMap;

use super::enclosing_index::EnclosingIndex;
use super::macro_origin::MacroOrigin;
use super::saved_origin_rows::SavedOriginRows;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct OwnCallOrigins {
    origins: FxHashMap<Span, MacroOrigin>,
    #[serde(skip)]
    enclosing_index: OnceCell<EnclosingIndex>,
}

impl OwnCallOrigins {
    pub fn at(&self, call_site: Span) -> Option<MacroOrigin> {
        self.origins.get(&call_site).cloned()
    }

    pub fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
        let index = self
            .enclosing_index
            .get_or_init(|| EnclosingIndex::of(self.origins.keys().copied()));
        self.at(index.smallest_enclosing(span)?)
    }

    pub fn record(&mut self, call_site: Span, origin: MacroOrigin) {
        self.enclosing_index.take();
        self.origins
            .entry(call_site)
            .and_modify(|outer| outer.through_prelude |= origin.through_prelude)
            .or_insert(origin);
    }

    pub fn into_rows(self) -> Vec<(Span, MacroOrigin)> {
        let mut rows: Vec<(Span, MacroOrigin)> = self.origins.into_iter().collect();
        rows.sort_by_key(|(call_site, _)| SavedOriginRows::key_of(*call_site));
        rows
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.origins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::OwnCallOrigins;
    use crate::expand::origins_fixture::OriginsFixture;

    struct OwnFixture {
        own: OwnCallOrigins,
    }

    impl OwnFixture {
        fn new() -> Self {
            Self {
                own: OwnCallOrigins::default(),
            }
        }

        fn record(&mut self, name: &str, start: usize, end: usize) {
            let call_site = OriginsFixture::span(1, start, end);
            self.own
                .record(call_site, OriginsFixture::origin(name, call_site));
        }

        fn enclosing_name(&self) -> Option<String> {
            OriginsFixture::name(self.own.enclosing(OriginsFixture::span(1, 13, 14)))
        }
    }

    #[test]
    fn a_recorded_origin_is_found_at_its_call_site() {
        let mut fixture = OwnFixture::new();
        fixture.record("own", 10, 20);
        let found = fixture.own.at(OriginsFixture::call_site());
        assert_eq!(OriginsFixture::name(found), Some("own".to_string()));
    }

    #[test]
    fn a_program_call_site_alone_encloses() {
        let mut fixture = OwnFixture::new();
        fixture.record("own", 10, 20);
        assert_eq!(fixture.enclosing_name(), Some("own".to_string()));
    }

    #[test]
    fn the_smallest_program_call_site_around_a_span_encloses_it() {
        let mut fixture = OwnFixture::new();
        fixture.record("wide", 5, 30);
        fixture.record("narrow", 12, 15);
        assert_eq!(fixture.enclosing_name(), Some("narrow".to_string()));
    }

    #[test]
    fn a_call_site_recorded_after_a_lookup_can_enclose_the_next_lookup() {
        let mut fixture = OwnFixture::new();
        fixture.record("wide", 5, 30);
        fixture.enclosing_name();
        fixture.record("narrow", 12, 15);
        assert_eq!(fixture.enclosing_name(), Some("narrow".to_string()));
    }

    #[test]
    fn a_program_call_site_away_from_a_span_does_not_enclose_it() {
        let mut fixture = OwnFixture::new();
        fixture.record("elsewhere", 40, 50);
        assert_eq!(fixture.enclosing_name(), None);
    }

    #[test]
    fn recording_a_call_site_twice_keeps_the_first_and_joins_through_prelude() {
        let mut fixture = OwnFixture::new();
        let call_site = OriginsFixture::call_site();
        fixture.record("first", 10, 20);
        let second = OriginsFixture::joined_origin("second", call_site);
        fixture.own.record(call_site, second);
        let kept = fixture
            .own
            .at(call_site)
            .expect("the call site is recorded");
        assert_eq!(kept.name.as_ref(), "first");
        assert!(kept.through_prelude);
    }
}
