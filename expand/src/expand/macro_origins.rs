use rustc_hash::FxHashMap;

use super::call_origins::CallOrigins;
use super::macro_origin::MacroOrigin;
use super::origin_row::{IntroducedNames, OriginRow};
use super::saved_call_origins::SavedCallOrigins;
use super::saved_index::SavedWords;
use super::saved_origins::SavedOrigins;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MacroOrigins {
    #[serde(skip)]
    saved: SavedOrigins,
    calls: CallOrigins,
    introduced: FxHashMap<(Span, String), Span>,
}

impl MacroOrigins {
    pub fn calls(&self) -> &CallOrigins {
        &self.calls
    }

    pub fn calls_mut(&mut self) -> &mut CallOrigins {
        &mut self.calls
    }

    pub fn introduced(&self, call_site: Span, name: &str) -> Option<Span> {
        let saved = self.saved.introduced(call_site, name);
        saved.or_else(|| self.introduced.get(&(call_site, name.to_string())).copied())
    }

    pub fn record_introduced(&mut self, call_site: Span, name: &str, span: Span) {
        self.introduced
            .entry((call_site, name.to_string()))
            .or_insert(span);
    }

    pub fn take_saved(&mut self) -> OriginTables {
        let calls = std::mem::take(&mut self.calls).into_own();
        let tables = OriginTables::of(calls.origins(), &self.introduced);
        self.introduced.clear();
        tables
    }

    pub fn adopt_saved(&mut self, saved: SavedOrigins) {
        self.calls = CallOrigins::over(SavedCallOrigins::new(saved));
        self.saved = saved;
    }

    #[cfg(test)]
    pub fn holds_no_own_origin(&self) -> bool {
        self.calls.holds_no_own_origin() && self.introduced.is_empty()
    }
}

#[derive(Default, serde::Serialize)]
pub struct OriginTables {
    rows: Vec<u8>,
    origins: Vec<u8>,
}

impl OriginTables {
    fn of(
        origins: &FxHashMap<Span, MacroOrigin>,
        introduced: &FxHashMap<(Span, String), Span>,
    ) -> Self {
        let mut names: FxHashMap<Span, IntroducedNames> = FxHashMap::default();
        for ((call_site, name), span) in introduced {
            names
                .entry(*call_site)
                .or_default()
                .push((name.clone(), *span));
        }
        let mut sorted: Vec<(&Span, &MacroOrigin)> = origins.iter().collect();
        sorted.sort_by_key(|(call_site, _)| OriginRow::key_of(**call_site));
        let mut tables = Self::default();
        for (call_site, origin) in sorted {
            let mut held = names.remove(call_site).unwrap_or_default();
            held.sort_by(|left, right| left.0.cmp(&right.0));
            tables.add(*call_site, origin, &held);
        }
        tables
    }

    fn add(&mut self, call_site: Span, origin: &MacroOrigin, names: &IntroducedNames) {
        let start = SavedWords::index(self.origins.len());
        let encoded = bincode::serialize(&(origin, names)).expect("every macro origin serializes");
        self.origins.extend(encoded);
        let [source, span_start, span_end] = OriginRow::key_of(call_site);
        let end = SavedWords::index(self.origins.len());
        SavedWords::write(&[source, span_start, span_end, start, end], &mut self.rows);
    }
}

#[cfg(test)]
mod tests {
    use super::{MacroOrigins, OriginTables};
    use crate::expand::macro_call_lookup::MacroCallLookup;
    use crate::expand::macro_origin::MacroOrigin;
    use crate::expand::saved_origins::SavedOrigins;
    use crate::source::{SourceId, Span};

    struct OriginsFixture;

    impl OriginsFixture {
        const CALL_START: usize = 10;
        const CALL_END: usize = 20;

        fn span(source: usize, start: usize, end: usize) -> Span {
            Span::new(SourceId::new(source), start..end)
        }

        fn call_site() -> Span {
            Self::span(1, Self::CALL_START, Self::CALL_END)
        }

        fn origin(name: &str, call_site: Span) -> MacroOrigin {
            MacroOrigin {
                name: name.into(),
                defined: Self::span(0, 0, 1),
                call_site,
                through_prelude: false,
            }
        }

        fn saved(origins: &mut MacroOrigins) -> SavedOrigins {
            let tables: OriginTables = origins.take_saved();
            let encoded = bincode::serialize(&tables).expect("the tables serialize");
            bincode::deserialize(encoded.leak()).expect("the tables load")
        }

        fn with_saved_origins() -> MacroOrigins {
            let mut origins = MacroOrigins::default();
            origins
                .calls_mut()
                .record(Self::call_site(), Self::origin("outer", Self::call_site()));
            let inner = Self::span(1, 12, 15);
            origins
                .calls_mut()
                .record(inner, Self::origin("inner", inner));
            let other_source = Self::span(2, 10, 20);
            origins
                .calls_mut()
                .record(other_source, Self::origin("elsewhere", other_source));
            origins.record_introduced(Self::call_site(), "helper", Self::span(0, 5, 6));
            let saved = Self::saved(&mut origins);
            let mut adopted = MacroOrigins::default();
            adopted.adopt_saved(saved);
            adopted
        }

        fn name(origin: Option<MacroOrigin>) -> Option<String> {
            origin.map(|found| found.name.to_string())
        }
    }

    #[test]
    fn taking_the_saved_origins_empties_the_table() {
        let mut origins = MacroOrigins::default();
        let call_site = OriginsFixture::call_site();
        origins
            .calls_mut()
            .record(call_site, OriginsFixture::origin("m", call_site));
        origins.record_introduced(call_site, "x", call_site);
        origins.take_saved();
        assert!(origins.holds_no_own_origin());
    }

    #[test]
    fn a_saved_origin_is_found_by_its_exact_call_site() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins
            .calls()
            .introducing_macro(OriginsFixture::call_site());
        assert_eq!(OriginsFixture::name(found), Some("outer".to_string()));
    }

    #[test]
    fn a_span_that_is_no_call_site_has_no_saved_origin() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(
            origins
                .calls()
                .introducing_macro(OriginsFixture::span(1, 11, 19))
                .is_none()
        );
    }

    #[test]
    fn the_smallest_saved_call_site_around_a_span_encloses_it() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins.calls().enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("inner".to_string()));
    }

    #[test]
    fn a_call_site_past_the_last_saved_one_has_no_origin() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(
            origins
                .calls()
                .introducing_macro(OriginsFixture::span(9, 0, 1))
                .is_none()
        );
    }

    #[test]
    fn a_span_outside_every_saved_call_site_is_not_enclosed() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(
            origins
                .calls()
                .enclosing(OriginsFixture::span(1, 30, 31))
                .is_none()
        );
    }

    #[test]
    fn a_saved_call_site_in_another_source_does_not_enclose() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(
            origins
                .calls()
                .enclosing(OriginsFixture::span(3, 13, 14))
                .is_none()
        );
    }

    #[test]
    fn a_saved_introduced_name_is_found_at_its_call_site() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins.introduced(OriginsFixture::call_site(), "helper");
        assert_eq!(found, Some(OriginsFixture::span(0, 5, 6)));
    }

    #[test]
    fn another_name_at_a_saved_call_site_is_not_introduced() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(
            origins
                .introduced(OriginsFixture::call_site(), "other")
                .is_none()
        );
    }

    #[test]
    fn a_saved_origin_keeps_its_name_and_joins_through_prelude() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::call_site();
        let mut own = OriginsFixture::origin("own", call_site);
        own.through_prelude = true;
        origins.calls_mut().record(call_site, own);
        let found = origins
            .calls()
            .introducing_macro(call_site)
            .expect("the call site is recorded");
        assert_eq!(found.name.as_ref(), "outer");
        assert!(found.through_prelude);
    }

    #[test]
    fn a_narrower_saved_call_site_encloses_over_a_wider_program_one() {
        let mut origins = OriginsFixture::with_saved_origins();
        let wide = OriginsFixture::span(1, 5, 30);
        origins
            .calls_mut()
            .record(wide, OriginsFixture::origin("wide", wide));
        let found = origins.calls().enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("inner".to_string()));
    }

    #[test]
    fn a_narrower_program_call_site_encloses_over_a_wider_saved_one() {
        let mut origins = OriginsFixture::with_saved_origins();
        let narrow = OriginsFixture::span(1, 13, 14);
        origins
            .calls_mut()
            .record(narrow, OriginsFixture::origin("narrow", narrow));
        let found = origins.calls().enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("narrow".to_string()));
    }

    #[test]
    fn an_enclosing_call_site_in_both_tables_joins_through_prelude() {
        let mut origins = OriginsFixture::with_saved_origins();
        let inner = OriginsFixture::span(1, 12, 15);
        let mut own = OriginsFixture::origin("own", inner);
        own.through_prelude = true;
        origins.calls_mut().record(inner, own);
        let found = origins
            .calls()
            .enclosing(OriginsFixture::span(1, 13, 14))
            .expect("the call site encloses the span");
        assert_eq!(found.name.as_ref(), "inner");
        assert!(found.through_prelude);
    }

    #[test]
    fn a_program_call_site_alone_encloses() {
        let mut origins = MacroOrigins::default();
        let call_site = OriginsFixture::call_site();
        origins
            .calls_mut()
            .record(call_site, OriginsFixture::origin("own", call_site));
        let found = origins.calls().enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("own".to_string()));
    }

    #[test]
    fn the_smallest_program_call_site_around_a_span_encloses_it() {
        let mut origins = MacroOrigins::default();
        let wide = OriginsFixture::span(1, 5, 30);
        origins
            .calls_mut()
            .record(wide, OriginsFixture::origin("wide", wide));
        let narrow = OriginsFixture::span(1, 12, 15);
        origins
            .calls_mut()
            .record(narrow, OriginsFixture::origin("narrow", narrow));
        let found = origins.calls().enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("narrow".to_string()));
    }

    #[test]
    fn a_call_site_recorded_after_a_lookup_can_enclose_the_next_lookup() {
        let mut origins = MacroOrigins::default();
        let wide = OriginsFixture::span(1, 5, 30);
        origins
            .calls_mut()
            .record(wide, OriginsFixture::origin("wide", wide));
        origins.calls().enclosing(OriginsFixture::span(1, 13, 14));
        let narrow = OriginsFixture::span(1, 12, 15);
        origins
            .calls_mut()
            .record(narrow, OriginsFixture::origin("narrow", narrow));
        let found = origins.calls().enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("narrow".to_string()));
    }

    #[test]
    fn a_program_call_site_away_from_a_span_does_not_enclose_it() {
        let mut origins = MacroOrigins::default();
        let elsewhere = OriginsFixture::span(1, 40, 50);
        origins
            .calls_mut()
            .record(elsewhere, OriginsFixture::origin("elsewhere", elsewhere));
        assert!(
            origins
                .calls()
                .enclosing(OriginsFixture::span(1, 13, 14))
                .is_none()
        );
    }

    #[test]
    fn a_joined_saved_origin_stays_joined_beside_an_unjoined_program_origin() {
        let call_site = OriginsFixture::call_site();
        let mut prelude = MacroOrigins::default();
        let mut joined = OriginsFixture::origin("outer", call_site);
        joined.through_prelude = true;
        prelude.calls_mut().record(call_site, joined);
        let mut origins = MacroOrigins::default();
        origins.adopt_saved(OriginsFixture::saved(&mut prelude));
        origins
            .calls_mut()
            .record(call_site, OriginsFixture::origin("own", call_site));
        let found = origins
            .calls()
            .introducing_macro(call_site)
            .expect("the call site is saved");
        assert!(found.through_prelude);
    }

    #[test]
    fn a_saved_origin_stays_unjoined_without_a_program_origin() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins
            .calls()
            .introducing_macro(OriginsFixture::call_site())
            .expect("the call site is saved");
        assert!(!found.through_prelude);
    }

    #[test]
    fn a_program_origin_alone_is_found() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::span(5, 1, 2);
        origins
            .calls_mut()
            .record(call_site, OriginsFixture::origin("own", call_site));
        assert_eq!(
            OriginsFixture::name(origins.calls().introducing_macro(call_site)),
            Some("own".to_string())
        );
    }

    #[test]
    fn a_saved_introduced_name_wins_over_a_program_one() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::call_site();
        origins.record_introduced(call_site, "helper", OriginsFixture::span(1, 1, 2));
        assert_eq!(
            origins.introduced(call_site, "helper"),
            Some(OriginsFixture::span(0, 5, 6))
        );
    }

    #[test]
    fn a_program_introduced_name_alone_is_found() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::span(5, 1, 2);
        let own = OriginsFixture::span(1, 1, 2);
        origins.record_introduced(call_site, "own", own);
        assert_eq!(origins.introduced(call_site, "own"), Some(own));
    }

    #[test]
    fn recording_a_call_site_twice_keeps_the_first_and_joins_through_prelude() {
        let mut origins = MacroOrigins::default();
        let call_site = OriginsFixture::call_site();
        origins
            .calls_mut()
            .record(call_site, OriginsFixture::origin("first", call_site));
        let mut second = OriginsFixture::origin("second", call_site);
        second.through_prelude = true;
        origins.calls_mut().record(call_site, second);
        let kept = origins
            .calls()
            .introducing_macro(call_site)
            .expect("the call site is recorded");
        assert_eq!(kept.name.as_ref(), "first");
        assert!(kept.through_prelude);
    }
}
