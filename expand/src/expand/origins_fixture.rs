use super::macro_origin::MacroOrigin;
use super::macro_origins::{MacroOrigins, OriginTables};
use super::saved_call_origins::SavedCallOrigins;
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

    pub fn saved(origins: &mut MacroOrigins) -> SavedOrigins {
        let tables: OriginTables = origins.take_saved();
        let encoded = bincode::serialize(&tables).expect("the tables serialize");
        bincode::deserialize(encoded.leak()).expect("the tables load")
    }

    pub fn with_saved_origins() -> MacroOrigins {
        let mut adopted = MacroOrigins::default();
        adopted.adopt_saved(Self::saved_tables());
        adopted
    }

    pub fn saved_calls_of(origin: MacroOrigin) -> SavedCallOrigins {
        let mut prelude = MacroOrigins::default();
        prelude.calls_mut().record(origin.call_site, origin);
        SavedCallOrigins::new(Self::saved(&mut prelude))
    }

    pub fn saved_calls() -> SavedCallOrigins {
        SavedCallOrigins::new(Self::saved_tables())
    }

    pub fn name(origin: Option<MacroOrigin>) -> Option<String> {
        origin.map(|found| found.name.to_string())
    }

    fn saved_tables() -> SavedOrigins {
        let mut origins = MacroOrigins::default();
        let outer = Self::call_site();
        origins
            .calls_mut()
            .record(outer, Self::origin("outer", outer));
        let inner = Self::span(1, 12, 15);
        origins
            .calls_mut()
            .record(inner, Self::origin("inner", inner));
        let other_source = Self::span(2, 10, 20);
        let elsewhere = Self::origin("elsewhere", other_source);
        origins.calls_mut().record(other_source, elsewhere);
        origins.record_introduced(outer, "helper", Self::span(0, 5, 6));
        Self::saved(&mut origins)
    }
}
