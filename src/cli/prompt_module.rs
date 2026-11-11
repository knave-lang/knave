use std::path::Path;

use super::module_of_sources::ModuleOfSources;
use super::program_file::ProgramFile;
use crate::source::SourceId;

pub struct PromptModule {
    anchor: SourceId,
    file: ProgramFile,
}

impl PromptModule {
    pub const PATH: &str = "<prompt>";

    pub fn new(anchor: SourceId, file: ProgramFile) -> Self {
        Self { anchor, file }
    }

    pub fn root(&self) -> &Path {
        self.file.root()
    }

    pub fn of(&self, inputs: Vec<SourceId>) -> ModuleOfSources {
        let mut sources = vec![self.anchor];
        sources.extend(inputs);
        ModuleOfSources {
            file: self.file.clone(),
            sources,
        }
    }
}
