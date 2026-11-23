use std::path::PathBuf;

use super::{Expander, PreludeState, Program};
use crate::core::ModulePath;
use crate::modules::ModuleSources;
use crate::source::SourceMap;

pub struct ExpandedModule;

impl ExpandedModule {
    pub const MODULE: &str = "fixture";

    pub fn of(text: &str) -> Program {
        Self::expanded_by(text, |sources, module_sources| {
            Expander::new(sources, module_sources)
        })
    }

    pub fn on_saved_prelude(text: &str) -> Program {
        Self::expanded_by(text, |sources, module_sources| {
            Expander::with_prelude(sources, module_sources, Self::saved_prelude())
        })
    }

    pub fn saved_prelude() -> PreludeState {
        PreludeState::load(PreludeState::build().save().leak()).expect("the saved prelude loads")
    }

    pub fn module_sources() -> ModuleSources {
        ModuleSources::new(PathBuf::new(), PathBuf::new())
    }

    fn expanded_by(
        text: &str,
        make: impl for<'s> FnOnce(&'s mut SourceMap, ModuleSources) -> Expander<'s>,
    ) -> Program {
        let mut sources = SourceMap::default();
        let entry = sources.add(Self::MODULE, text);
        make(&mut sources, Self::module_sources())
            .expand(ModulePath::new(Self::MODULE), entry)
            .unwrap_or_else(|_| panic!("the program expands"))
    }
}
