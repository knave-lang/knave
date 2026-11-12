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
