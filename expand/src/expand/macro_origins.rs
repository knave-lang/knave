use std::ops::Range;

use rustc_hash::FxHashMap;

use super::macro_call::MacroOrigin;
use super::saved_index::SavedWords;
use crate::source::{SourceId, Span};

type IntroducedNames = Vec<(String, Span)>;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MacroOrigins {
    #[serde(skip)]
    saved: SavedOrigins,
    origins: FxHashMap<Span, MacroOrigin>,
    introduced: FxHashMap<(Span, String), Span>,
}

impl MacroOrigins {
    pub fn get(&self, call_site: Span) -> Option<MacroOrigin> {
        let own = self.origins.get(&call_site).cloned();
        match (self.saved.origin_at(call_site), own) {
            (Some(mut saved), Some(own)) => {
                saved.through_prelude |= own.through_prelude;
                Some(saved)
            }
            (saved, own) => saved.or(own),
        }
    }

    pub fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
        let saved = self.saved.enclosing(span);
        let smallest = saved
            .into_iter()
            .chain(self.own_enclosing(span))
            .min_by_key(Self::call_site_length)?;
        self.get(smallest.call_site)
    }

    fn own_enclosing(&self, span: Span) -> Option<MacroOrigin> {
        self.origins
            .values()
            .filter(|origin| origin.call_site.contains(span))
            .min_by_key(|origin| Self::call_site_length(origin))
            .cloned()
    }

    fn call_site_length(origin: &MacroOrigin) -> usize {
        origin.call_site.range().len()
    }

    pub fn introduced(&self, call_site: Span, name: &str) -> Option<Span> {
        let saved = self.saved.introduced(call_site, name);
        saved.or_else(|| self.introduced.get(&(call_site, name.to_string())).copied())
    }

    pub fn record(&mut self, call_site: Span, origin: MacroOrigin) {
        self.origins
            .entry(call_site)
            .and_modify(|outer| outer.through_prelude |= origin.through_prelude)
            .or_insert(origin);
    }

    pub fn record_introduced(&mut self, call_site: Span, name: &str, span: Span) {
        self.introduced
            .entry((call_site, name.to_string()))
            .or_insert(span);
    }

    pub fn forget_introduced_names(&mut self) {
        self.introduced = FxHashMap::default();
    }

    pub fn take_saved(&mut self) -> OriginTables {
        let tables = OriginTables::of(&self.origins, &self.introduced);
        self.origins.clear();
        self.introduced.clear();
        tables
    }

    pub fn adopt_saved(&mut self, saved: SavedOrigins) {
        self.saved = saved;
    }

    #[cfg(test)]
    pub fn holds_no_introduced_name(&self) -> bool {
        self.introduced.is_empty()
    }

    #[cfg(test)]
    pub fn holds_no_own_origin(&self) -> bool {
        self.origins.is_empty() && self.introduced.is_empty()
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

struct OriginRow;

impl OriginRow {
    const WORDS: usize = 5;
    const SIZE: usize = Self::WORDS * SavedWords::SIZE;
    const KEY_WORDS: usize = 3;
    const START: usize = 3;
    const END: usize = 4;

    fn key_of(span: Span) -> [u32; Self::KEY_WORDS] {
        let range = span.range();
        [
            SavedWords::index(span.source().index()),
            SavedWords::index(range.start),
            SavedWords::index(range.end),
        ]
    }

    fn word(row: &[u8], field: usize) -> u32 {
        SavedWords::read(&row[field * SavedWords::SIZE..])
    }

    fn key(row: &[u8]) -> [u32; Self::KEY_WORDS] {
        [Self::word(row, 0), Self::word(row, 1), Self::word(row, 2)]
    }

    fn call_site(row: &[u8]) -> Span {
        let [source, start, end] = Self::key(row);
        Span::new(SourceId::new(source as usize), start as usize..end as usize)
    }
}

#[derive(Default)]
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
    fn origin_at(&self, call_site: Span) -> Option<MacroOrigin> {
        let row = self.row_at(call_site)?;
        Some(self.decode(row).0)
    }

    fn introduced(&self, call_site: Span, name: &str) -> Option<Span> {
        let (_, names) = self.decode(self.row_at(call_site)?);
        let found = names.iter().find(|(held, _)| held == name);
        found.map(|(_, span)| *span)
    }

    fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
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

#[cfg(test)]
mod tests {
    use super::{MacroOrigins, OriginTables, SavedOrigins};
    use crate::expand::macro_call::MacroOrigin;
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
            origins.record(Self::call_site(), Self::origin("outer", Self::call_site()));
            let inner = Self::span(1, 12, 15);
            origins.record(inner, Self::origin("inner", inner));
            let other_source = Self::span(2, 10, 20);
            origins.record(other_source, Self::origin("elsewhere", other_source));
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
        origins.record(call_site, OriginsFixture::origin("m", call_site));
        origins.record_introduced(call_site, "x", call_site);
        origins.take_saved();
        assert!(origins.holds_no_own_origin());
    }

    #[test]
    fn forgetting_the_introduced_names_keeps_the_origins() {
        let mut origins = MacroOrigins::default();
        let call_site = OriginsFixture::call_site();
        origins.record(call_site, OriginsFixture::origin("m", call_site));
        origins.record_introduced(call_site, "x", call_site);
        origins.forget_introduced_names();
        assert!(origins.introduced(call_site, "x").is_none());
        assert!(origins.get(call_site).is_some());
    }

    #[test]
    fn a_saved_origin_is_found_by_its_exact_call_site() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins.get(OriginsFixture::call_site());
        assert_eq!(OriginsFixture::name(found), Some("outer".to_string()));
    }

    #[test]
    fn a_span_that_is_no_call_site_has_no_saved_origin() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(origins.get(OriginsFixture::span(1, 11, 19)).is_none());
    }

    #[test]
    fn the_smallest_saved_call_site_around_a_span_encloses_it() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins.enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("inner".to_string()));
    }

    #[test]
    fn a_call_site_past_the_last_saved_one_has_no_origin() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(origins.get(OriginsFixture::span(9, 0, 1)).is_none());
    }

    #[test]
    fn a_span_outside_every_saved_call_site_is_not_enclosed() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(origins.enclosing(OriginsFixture::span(1, 30, 31)).is_none());
    }

    #[test]
    fn a_saved_call_site_in_another_source_does_not_enclose() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(origins.enclosing(OriginsFixture::span(3, 13, 14)).is_none());
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
        origins.record(call_site, own);
        let found = origins.get(call_site).expect("the call site is recorded");
        assert_eq!(found.name.as_ref(), "outer");
        assert!(found.through_prelude);
    }

    #[test]
    fn a_narrower_saved_call_site_encloses_over_a_wider_program_one() {
        let mut origins = OriginsFixture::with_saved_origins();
        let wide = OriginsFixture::span(1, 5, 30);
        origins.record(wide, OriginsFixture::origin("wide", wide));
        let found = origins.enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("inner".to_string()));
    }

    #[test]
    fn a_narrower_program_call_site_encloses_over_a_wider_saved_one() {
        let mut origins = OriginsFixture::with_saved_origins();
        let narrow = OriginsFixture::span(1, 13, 14);
        origins.record(narrow, OriginsFixture::origin("narrow", narrow));
        let found = origins.enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("narrow".to_string()));
    }

    #[test]
    fn an_enclosing_call_site_in_both_tables_joins_through_prelude() {
        let mut origins = OriginsFixture::with_saved_origins();
        let inner = OriginsFixture::span(1, 12, 15);
        let mut own = OriginsFixture::origin("own", inner);
        own.through_prelude = true;
        origins.record(inner, own);
        let found = origins
            .enclosing(OriginsFixture::span(1, 13, 14))
            .expect("the call site encloses the span");
        assert_eq!(found.name.as_ref(), "inner");
        assert!(found.through_prelude);
    }

    #[test]
    fn a_program_call_site_alone_encloses() {
        let mut origins = MacroOrigins::default();
        let call_site = OriginsFixture::call_site();
        origins.record(call_site, OriginsFixture::origin("own", call_site));
        let found = origins.enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("own".to_string()));
    }

    #[test]
    fn the_smallest_program_call_site_around_a_span_encloses_it() {
        let mut origins = MacroOrigins::default();
        let wide = OriginsFixture::span(1, 5, 30);
        origins.record(wide, OriginsFixture::origin("wide", wide));
        let narrow = OriginsFixture::span(1, 12, 15);
        origins.record(narrow, OriginsFixture::origin("narrow", narrow));
        let found = origins.enclosing(OriginsFixture::span(1, 13, 14));
        assert_eq!(OriginsFixture::name(found), Some("narrow".to_string()));
    }

    #[test]
    fn a_program_call_site_away_from_a_span_does_not_enclose_it() {
        let mut origins = MacroOrigins::default();
        let elsewhere = OriginsFixture::span(1, 40, 50);
        origins.record(elsewhere, OriginsFixture::origin("elsewhere", elsewhere));
        assert!(origins.enclosing(OriginsFixture::span(1, 13, 14)).is_none());
    }

    #[test]
    fn a_saved_origin_stays_unjoined_without_a_program_origin() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins
            .get(OriginsFixture::call_site())
            .expect("the call site is saved");
        assert!(!found.through_prelude);
    }

    #[test]
    fn a_program_origin_alone_is_found() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::span(5, 1, 2);
        origins.record(call_site, OriginsFixture::origin("own", call_site));
        assert_eq!(
            OriginsFixture::name(origins.get(call_site)),
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
        origins.record(call_site, OriginsFixture::origin("first", call_site));
        let mut second = OriginsFixture::origin("second", call_site);
        second.through_prelude = true;
        origins.record(call_site, second);
        let kept = origins.get(call_site).expect("the call site is recorded");
        assert_eq!(kept.name.as_ref(), "first");
        assert!(kept.through_prelude);
    }
}
