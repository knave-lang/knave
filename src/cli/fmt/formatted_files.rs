use std::path::{Path, PathBuf};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::cli::program_file::ProgramFile;
use crate::diagnostic::Diagnostics;
use crate::reader::Reader;
use crate::source::{SourceId, SourceMap};

pub struct FormattedFiles {
    files: FxHashSet<PathBuf>,
}

impl FormattedFiles {
    pub fn of(files: &[PathBuf]) -> Self {
        Self {
            files: files.iter().map(|file| Self::key(file)).collect(),
        }
    }

    pub fn none() -> Self {
        Self::of(&[])
    }

    pub fn includes(&self, path: &Path) -> bool {
        self.files.contains(&Self::key(path))
    }

    pub fn without_unreadable(
        &self,
        sources: &SourceMap,
        mut diagnostics: Diagnostics,
    ) -> Diagnostics {
        let mut unreadable_by_source: FxHashMap<SourceId, bool> = FxHashMap::default();
        diagnostics.retain(|diagnostic| {
            let source = diagnostic.primary().span().source();
            let unreadable = unreadable_by_source
                .entry(source)
                .or_insert_with(|| self.is_unreadable_formatted_file(sources, source));
            !*unreadable
        });
        diagnostics
    }

    fn is_unreadable_formatted_file(&self, sources: &SourceMap, source: SourceId) -> bool {
        let file = sources.file(source);
        self.includes(Path::new(file.name())) && Reader::new(file).read().is_err()
    }

    fn key(path: &Path) -> PathBuf {
        ProgramFile::canonical(path).unwrap_or_else(|| path.to_path_buf())
    }
}
