use crate::expand::PreludeState;
use crate::format::{DocstringSyntax, SourceFormatter};

pub struct EmbeddedPrelude;

impl EmbeddedPrelude {
    const SAVED: &'static [u8] = include_bytes!(env!("PRELUDE_STATE"));

    pub fn state() -> PreludeState {
        PreludeState::load(Self::SAVED).expect("the embedded prelude state loads")
    }

    pub fn source_formatter() -> SourceFormatter {
        SourceFormatter::new(DocstringSyntax::of_prelude(&Self::state()))
    }
}
