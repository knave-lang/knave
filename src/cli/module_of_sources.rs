use super::program_file::ProgramFile;
use crate::source::SourceId;

pub struct ModuleOfSources {
    pub file: ProgramFile,
    pub sources: Vec<SourceId>,
}

impl ModuleOfSources {
    pub fn single(file: ProgramFile, entry: SourceId) -> Self {
        Self {
            file,
            sources: vec![entry],
        }
    }

    pub fn anchor(&self) -> SourceId {
        self.sources[0]
    }
}
