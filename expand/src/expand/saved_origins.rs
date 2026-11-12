use super::call_origins::CallOrigins;
use super::introduced_names::IntroducedNames;
use super::macro_origins::MacroOrigins;
use super::saved_call_origins::SavedCallOrigins;
use super::saved_introductions::SavedIntroductions;
use super::saved_origin_rows::SavedOriginRows;

#[derive(Default, serde::Deserialize)]
#[serde(transparent, bound(deserialize = "'de: 'static"))]
pub struct SavedOrigins {
    rows: SavedOriginRows,
}

impl SavedOrigins {
    pub fn into_macro_origins(self) -> MacroOrigins {
        MacroOrigins {
            calls: CallOrigins::over(SavedCallOrigins::new(self.rows)),
            introduced: IntroducedNames::over(SavedIntroductions::new(self.rows)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::expand::macro_call_lookup::MacroCallLookup;
    use crate::expand::origins_fixture::OriginsFixture;

    #[test]
    fn its_macro_origins_answer_saved_calls_and_names() {
        let origins = OriginsFixture::with_saved_origins();
        let call_site = OriginsFixture::call_site();
        let found = origins.calls.introducing_macro(call_site);
        assert_eq!(OriginsFixture::name(found), Some("outer".to_string()));
        let written = origins.introduced.at(call_site, "helper");
        assert_eq!(written, Some(OriginsFixture::span(0, 5, 6)));
    }

    #[test]
    fn its_macro_origins_start_with_empty_own_layers() {
        let origins = OriginsFixture::with_saved_origins();
        assert!(origins.calls.holds_no_own_origin());
        assert!(origins.introduced.holds_no_own_name());
    }
}
