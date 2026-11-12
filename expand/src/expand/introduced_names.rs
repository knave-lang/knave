use super::own_introductions::OwnIntroductions;
use super::saved_introductions::SavedIntroductions;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct IntroducedNames {
    #[serde(skip)]
    saved: SavedIntroductions,
    own: OwnIntroductions,
}

impl IntroducedNames {
    pub fn over(saved: SavedIntroductions) -> Self {
        Self {
            saved,
            own: OwnIntroductions::default(),
        }
    }

    pub fn at(&self, call_site: Span, name: &str) -> Option<Span> {
        let saved = self.saved.at(call_site, name);
        saved.or_else(|| self.own.at(call_site, name))
    }

    pub fn record(&mut self, call_site: Span, name: &str, written: Span) {
        self.own.record(call_site, name, written);
    }

    pub fn into_own(self) -> OwnIntroductions {
        self.own
    }

    #[cfg(test)]
    pub fn holds_no_own_name(&self) -> bool {
        self.own.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::expand::origins_fixture::OriginsFixture;

    #[test]
    fn a_saved_introduced_name_is_found_at_its_call_site() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins.introduced.at(OriginsFixture::call_site(), "helper");
        assert_eq!(found, Some(OriginsFixture::span(0, 5, 6)));
    }

    #[test]
    fn another_name_at_a_saved_call_site_is_not_introduced() {
        let origins = OriginsFixture::with_saved_origins();
        let found = origins.introduced.at(OriginsFixture::call_site(), "other");
        assert!(found.is_none());
    }

    #[test]
    fn a_saved_introduced_name_wins_over_a_program_one() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::call_site();
        let own = OriginsFixture::span(1, 1, 2);
        origins.introduced.record(call_site, "helper", own);
        assert_eq!(
            origins.introduced.at(call_site, "helper"),
            Some(OriginsFixture::span(0, 5, 6))
        );
    }

    #[test]
    fn recording_an_introduced_name_twice_keeps_the_first() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::call_site();
        let first = OriginsFixture::span(1, 1, 2);
        origins.introduced.record(call_site, "x", first);
        origins
            .introduced
            .record(call_site, "x", OriginsFixture::span(1, 3, 4));
        assert_eq!(origins.introduced.at(call_site, "x"), Some(first));
    }

    #[test]
    fn a_program_introduced_name_alone_is_found() {
        let mut origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::span(5, 1, 2);
        let own = OriginsFixture::span(1, 1, 2);
        origins.introduced.record(call_site, "own", own);
        assert_eq!(origins.introduced.at(call_site, "own"), Some(own));
    }
}
