use rustc_hash::FxHashMap;

use super::introduced_name::IntroducedName;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct OwnIntroductions {
    names: FxHashMap<(Span, String), Span>,
}

impl OwnIntroductions {
    pub fn at(&self, call_site: Span, name: &str) -> Option<Span> {
        self.names.get(&(call_site, name.to_string())).copied()
    }

    pub fn record(&mut self, call_site: Span, name: &str, written: Span) {
        self.names
            .entry((call_site, name.to_string()))
            .or_insert(written);
    }

    pub fn into_names_by_call_site(self) -> FxHashMap<Span, Vec<IntroducedName>> {
        let mut names_by_call_site: FxHashMap<Span, Vec<IntroducedName>> = FxHashMap::default();
        for ((call_site, name), written) in self.names {
            names_by_call_site
                .entry(call_site)
                .or_default()
                .push(IntroducedName { name, written });
        }
        for names in names_by_call_site.values_mut() {
            names.sort_by(|left, right| left.name.cmp(&right.name));
        }
        names_by_call_site
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::OwnIntroductions;
    use crate::expand::origins_fixture::OriginsFixture;

    #[test]
    fn a_recorded_name_is_found_at_its_call_site() {
        let mut own = OwnIntroductions::default();
        let written = OriginsFixture::span(1, 1, 2);
        own.record(OriginsFixture::call_site(), "x", written);
        assert_eq!(own.at(OriginsFixture::call_site(), "x"), Some(written));
    }

    #[test]
    fn a_name_recorded_at_another_call_site_is_not_found() {
        let mut own = OwnIntroductions::default();
        own.record(
            OriginsFixture::span(2, 1, 2),
            "x",
            OriginsFixture::span(1, 1, 2),
        );
        assert!(own.at(OriginsFixture::call_site(), "x").is_none());
    }

    #[test]
    fn recording_a_name_twice_keeps_the_first_place_it_was_written() {
        let mut own = OwnIntroductions::default();
        let call_site = OriginsFixture::call_site();
        let first = OriginsFixture::span(1, 1, 2);
        own.record(call_site, "x", first);
        own.record(call_site, "x", OriginsFixture::span(1, 3, 4));
        assert_eq!(own.at(call_site, "x"), Some(first));
    }

    #[test]
    fn the_names_of_each_call_site_stay_apart() {
        let mut own = OwnIntroductions::default();
        let other = OriginsFixture::span(2, 1, 2);
        own.record(
            OriginsFixture::call_site(),
            "a",
            OriginsFixture::span(1, 1, 2),
        );
        own.record(other, "c", OriginsFixture::span(1, 5, 6));
        let by_call_site = own.into_names_by_call_site();
        let names: Vec<&str> = by_call_site[&other]
            .iter()
            .map(|held| held.name.as_str())
            .collect();
        assert_eq!(names, ["c"]);
    }

    #[test]
    fn the_names_of_a_call_site_come_in_name_order() {
        let mut own = OwnIntroductions::default();
        let call_site = OriginsFixture::call_site();
        for (offset, name) in ["e", "d", "c", "b", "a"].into_iter().enumerate() {
            own.record(call_site, name, OriginsFixture::span(1, offset, offset + 1));
        }
        let by_call_site = own.into_names_by_call_site();
        let names: Vec<&str> = by_call_site[&call_site]
            .iter()
            .map(|held| held.name.as_str())
            .collect();
        assert_eq!(names, ["a", "b", "c", "d", "e"]);
    }
}
