use std::path::PathBuf;

use super::{Expander, Program};
use crate::core::ModulePath;
use crate::modules::ModuleSources;
use crate::source::SourceMap;

pub struct ExpandedModule;

impl ExpandedModule {
    const MODULE: &str = "fixture";

    pub fn of(text: &str) -> Program {
        let mut sources = SourceMap::default();
        let entry = sources.add(Self::MODULE, text);
        let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
        Expander::new(&mut sources, module_sources)
            .expand(ModulePath::new(Self::MODULE), entry)
            .unwrap_or_else(|_| panic!("the program expands"))
    }
}
