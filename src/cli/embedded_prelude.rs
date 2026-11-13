use std::sync::OnceLock;

use crate::expand::{PreludeState, SavedPrelude};
use crate::format::{DocstringSyntax, SourceFormatter};
use crate::project::ProjectVocabulary;

pub struct EmbeddedPrelude;

impl EmbeddedPrelude {
    const SAVED: &'static [u8] = include_bytes!(env!("PRELUDE_STATE"));

    pub fn state() -> PreludeState {
        SavedPrelude::state(Self::SAVED).expect("the embedded prelude state loads")
    }

    pub fn project_vocabulary() -> &'static ProjectVocabulary {
        static VOCABULARY: OnceLock<ProjectVocabulary> = OnceLock::new();
        VOCABULARY.get_or_init(|| SavedPrelude::project_vocabulary(Self::SAVED))
    }

    pub fn source_formatter() -> SourceFormatter {
        SourceFormatter::new(DocstringSyntax::of_prelude(&Self::state()))
    }
}
