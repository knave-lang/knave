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
    use super::MacroOrigins;
    use crate::expand::origins_fixture::OriginsFixture;

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
    fn a_saved_introduced_name_is_found_at_its_call_site() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins.introduced(OriginsFixture::call_site(), "helper");
        assert_eq!(found, Some(OriginsFixture::span(0, 5, 6)));
    }

    #[test]
    fn another_name_at_a_saved_call_site_is_not_introduced() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins.introduced(OriginsFixture::call_site(), "other");
        assert!(found.is_none());
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
    fn recording_an_introduced_name_twice_keeps_the_first() {
        let mut origins = MacroOrigins::default();
        let call_site = OriginsFixture::call_site();
        let first = OriginsFixture::span(1, 1, 2);
        origins.record_introduced(call_site, "x", first);
        origins.record_introduced(call_site, "x", OriginsFixture::span(1, 3, 4));
        assert_eq!(origins.introduced(call_site, "x"), Some(first));
    }

    #[test]
    fn a_program_introduced_name_alone_is_found() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::span(5, 1, 2);
        let own = OriginsFixture::span(1, 1, 2);
        origins.record_introduced(call_site, "own", own);
        assert_eq!(origins.introduced(call_site, "own"), Some(own));
    }
}
