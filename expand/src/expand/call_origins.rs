use std::rc::Rc;

use super::macro_call_lookup::MacroCallLookup;
use super::macro_origin::MacroOrigin;
use super::own_call_origins::OwnCallOrigins;
use super::saved_call_origins::SavedCallOrigins;
use crate::source::Span;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct CallOrigins {
    #[serde(skip)]
    saved: SavedCallOrigins,
    own: OwnCallOrigins,
}

impl CallOrigins {
    pub fn over(saved: SavedCallOrigins) -> Self {
        Self {
            saved,
            own: OwnCallOrigins::default(),
        }
    }

    pub fn record(&mut self, call_site: Span, origin: MacroOrigin) {
        self.own.record(call_site, origin);
    }

    pub fn into_own(self) -> OwnCallOrigins {
        self.own
    }

    #[cfg(test)]
    pub fn holds_no_own_origin(&self) -> bool {
        self.own.origins().is_empty()
    }

    fn call_site_length(origin: &MacroOrigin) -> usize {
        origin.call_site.range().len()
    }
}

impl dyn MacroCallLookup {
    pub fn empty() -> Rc<dyn MacroCallLookup> {
        Rc::new(CallOrigins::default())
    }
}

impl MacroCallLookup for CallOrigins {
    fn introducing_macro(&self, span: Span) -> Option<MacroOrigin> {
        match (self.saved.at(span), self.own.at(span)) {
            (Some(mut saved), Some(own)) => {
                saved.through_prelude |= own.through_prelude;
                Some(saved)
            }
            (saved, own) => saved.or(own),
        }
    }

    fn enclosing(&self, span: Span) -> Option<MacroOrigin> {
        let smallest = self
            .saved
            .enclosing(span)
            .into_iter()
            .chain(self.own.enclosing(span))
            .min_by_key(Self::call_site_length)?;
        self.introducing_macro(smallest.call_site)
    }
}

#[cfg(test)]
mod tests {
    use super::CallOrigins;
    use crate::expand::macro_call_lookup::MacroCallLookup;
    use crate::expand::origins_fixture::OriginsFixture;
    use crate::source::Span;

    struct CallOriginsFixture {
        origins: CallOrigins,
    }

    impl CallOriginsFixture {
        fn over_saved() -> Self {
            Self {
                origins: CallOrigins::over(OriginsFixture::saved_calls()),
            }
        }

        fn record(&mut self, name: &str, call_site: Span) {
            self.origins
                .record(call_site, OriginsFixture::origin(name, call_site));
        }

        fn record_joined(&mut self, name: &str, call_site: Span) {
            self.origins
                .record(call_site, OriginsFixture::joined_origin(name, call_site));
        }

        fn enclosing_name(&self) -> Option<String> {
            OriginsFixture::name(self.origins.enclosing(OriginsFixture::span(1, 13, 14)))
        }
    }

    #[test]
    fn a_saved_origin_keeps_its_name_and_joins_through_prelude() {
        let mut fixture = CallOriginsFixture::over_saved();
        let call_site = OriginsFixture::call_site();
        fixture.record_joined("own", call_site);
        let found = fixture.origins.introducing_macro(call_site);
        let found = found.expect("the call site is recorded");
        assert_eq!(found.name.as_ref(), "outer");
        assert!(found.through_prelude);
    }

    #[test]
    fn a_joined_saved_origin_stays_joined_beside_an_unjoined_program_origin() {
        let call_site = OriginsFixture::call_site();
        let joined = OriginsFixture::joined_origin("outer", call_site);
        let mut fixture = CallOriginsFixture {
            origins: CallOrigins::over(OriginsFixture::saved_calls_of(joined)),
        };
        fixture.record("own", call_site);
        let found = fixture.origins.introducing_macro(call_site);
        assert!(found.expect("the call site is saved").through_prelude);
    }

    #[test]
    fn a_saved_origin_stays_unjoined_without_a_program_origin() {
        let fixture = CallOriginsFixture::over_saved();
        let found = fixture
            .origins
            .introducing_macro(OriginsFixture::call_site())
            .expect("the call site is saved");
        assert!(!found.through_prelude);
    }

    #[test]
    fn a_program_origin_alone_is_found() {
        let mut fixture = CallOriginsFixture::over_saved();
        let call_site = OriginsFixture::span(5, 1, 2);
        fixture.record("own", call_site);
        let found = fixture.origins.introducing_macro(call_site);
        assert_eq!(OriginsFixture::name(found), Some("own".to_string()));
    }

    #[test]
    fn a_narrower_saved_call_site_encloses_over_a_wider_program_one() {
        let mut fixture = CallOriginsFixture::over_saved();
        fixture.record("wide", OriginsFixture::span(1, 5, 30));
        assert_eq!(fixture.enclosing_name(), Some("inner".to_string()));
    }

    #[test]
    fn a_narrower_program_call_site_encloses_over_a_wider_saved_one() {
        let mut fixture = CallOriginsFixture::over_saved();
        fixture.record("narrow", OriginsFixture::span(1, 13, 14));
        assert_eq!(fixture.enclosing_name(), Some("narrow".to_string()));
    }

    #[test]
    fn an_enclosing_call_site_in_both_tables_joins_through_prelude() {
        let mut fixture = CallOriginsFixture::over_saved();
        fixture.record_joined("own", OriginsFixture::span(1, 12, 15));
        let found = fixture
            .origins
            .enclosing(OriginsFixture::span(1, 13, 14))
            .expect("the call site encloses the span");
        assert_eq!(found.name.as_ref(), "inner");
        assert!(found.through_prelude);
    }

    #[test]
    fn an_empty_lookup_names_no_macro() {
        let lookup = <dyn MacroCallLookup>::empty();
        let call_site = OriginsFixture::call_site();
        assert!(lookup.introducing_macro(call_site).is_none());
        assert!(lookup.enclosing(call_site).is_none());
    }

    #[test]
    fn its_own_layer_keeps_what_it_recorded_and_not_the_saved_layer() {
        let mut fixture = CallOriginsFixture::over_saved();
        let call_site = OriginsFixture::span(5, 1, 2);
        fixture.record("own", call_site);
        let own = fixture.origins.into_own();
        assert_eq!(
            OriginsFixture::name(own.at(call_site)),
            Some("own".to_string())
        );
        assert!(own.at(OriginsFixture::call_site()).is_none());
    }
}
