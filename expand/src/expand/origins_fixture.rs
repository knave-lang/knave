use super::macro_origin::MacroOrigin;
use super::macro_origins::MacroOrigins;
use super::origin_tables::OriginTables;
use super::saved_call_origins::SavedCallOrigins;
use super::saved_introductions::SavedIntroductions;
use super::saved_origin_rows::SavedOriginRows;
use super::saved_origins::SavedOrigins;
use crate::source::{SourceId, Span};

pub struct OriginsFixture;

impl OriginsFixture {
    const CALL_START: usize = 10;
    const CALL_END: usize = 20;

    pub fn span(source: usize, start: usize, end: usize) -> Span {
        Span::new(SourceId::new(source), start..end)
    }

    pub fn call_site() -> Span {
        Self::span(1, Self::CALL_START, Self::CALL_END)
    }

    pub fn origin(name: &str, call_site: Span) -> MacroOrigin {
        MacroOrigin {
            name: name.into(),
            defined: Self::span(0, 0, 1),
            call_site,
            through_prelude: false,
        }
    }

    pub fn joined_origin(name: &str, call_site: Span) -> MacroOrigin {
        MacroOrigin {
            through_prelude: true,
            ..Self::origin(name, call_site)
        }
    }

    pub fn with_saved_origins() -> MacroOrigins {
        let saved: SavedOrigins = Self::load(Self::saved_program());
        saved.into_macro_origins()
    }

    pub fn saved_calls_of(origin: MacroOrigin) -> SavedCallOrigins {
        let mut prelude = MacroOrigins::default();
        prelude.calls.record(origin.call_site, origin);
        SavedCallOrigins::new(Self::load(prelude))
    }

    pub fn saved_calls() -> SavedCallOrigins {
        SavedCallOrigins::new(Self::saved_rows())
    }

    pub fn saved_introductions() -> SavedIntroductions {
        SavedIntroductions::new(Self::saved_rows())
    }

    pub fn saved_rows() -> SavedOriginRows {
        Self::load(Self::saved_program())
    }

    pub fn saved_rows_of(origins: MacroOrigins) -> SavedOriginRows {
        Self::load(origins)
    }

    pub fn name(origin: Option<MacroOrigin>) -> Option<String> {
        origin.map(|found| found.name.to_string())
    }

    fn saved_program() -> MacroOrigins {
        let mut origins = MacroOrigins::default();
        let outer = Self::call_site();
        origins.calls.record(outer, Self::origin("outer", outer));
        let inner = Self::span(1, 12, 15);
        origins.calls.record(inner, Self::origin("inner", inner));
        let other_source = Self::span(2, 10, 20);
        let elsewhere = Self::origin("elsewhere", other_source);
        origins.calls.record(other_source, elsewhere);
        origins
            .introduced
            .record(outer, "helper", Self::span(0, 5, 6));
        origins
    }

    fn load<Saved: serde::Deserialize<'static>>(origins: MacroOrigins) -> Saved {
        let tables = OriginTables::of_own_layers(origins);
        let encoded = bincode::serialize(&tables).expect("the tables serialize");
        bincode::deserialize(encoded.leak()).expect("the tables load")
    }
}
