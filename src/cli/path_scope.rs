use std::path::{Path, PathBuf};

use super::program_file::ProgramFile;
use crate::diagnostic::Diagnostics;
use crate::source::SourceMap;

pub struct PathScope {
    path: Option<PathBuf>,
}

impl PathScope {
    pub fn whole_project() -> Self {
        Self { path: None }
    }

    pub fn under(path: &Path) -> Self {
        Self {
            path: ProgramFile::canonical(path),
        }
    }

    pub fn kept(&self, sources: &SourceMap, diagnostics: Diagnostics) -> Diagnostics {
        let Some(path) = &self.path else {
            return diagnostics;
        };
        let mut kept = diagnostics;
        kept.retain(|diagnostic| {
            let file = sources.file(diagnostic.primary().span().source());
            ProgramFile::canonical(Path::new(file.name()))
                .is_some_and(|written| written.starts_with(path))
        });
        kept
    }
}
