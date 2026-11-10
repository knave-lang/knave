use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::failure::TaskFailure;
use crate::functions::SourceTree;
use crate::parsed_source::ParsedSource;
use crate::source_file::SourceFile;
use crate::workspace::CargoWorkspace;

pub struct ProductionSources {
    root: PathBuf,
}

impl ProductionSources {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn files(&self) -> Result<Vec<SourceFile>, TaskFailure> {
        SourceTree::new(&self.root, self.directories()?).files()
    }

    pub fn parsed(&self) -> Result<Vec<ParsedSource>, TaskFailure> {
        let directories = self.directories()?;
        let crate_directories: Vec<String> = directories
            .iter()
            .map(|directory| self.relative(directory))
            .collect();
        let files = SourceTree::new(&self.root, directories).files()?;
        Ok(ParsedSource::all_production(files, &crate_directories))
    }

    fn directories(&self) -> Result<BTreeSet<PathBuf>, TaskFailure> {
        Ok(CargoWorkspace::read(&self.root)?.source_directories_with_tooling())
    }

    fn relative(&self, directory: &Path) -> String {
        directory
            .strip_prefix(&self.root)
            .unwrap_or(directory)
            .display()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::ProductionSources;
    use crate::workspace::WorkspaceFixture;

    struct Repository;

    impl Repository {
        const CRATE_DIRECTORIES: [&str; 3] = ["src", "expand/src", "xtask/src"];

        fn sources() -> ProductionSources {
            ProductionSources::new(WorkspaceFixture::repository_root())
        }
    }

    #[test]
    fn files_cover_each_crate_and_the_tooling() {
        let files = Repository::sources().files().expect("the repository reads");
        for directory in Repository::CRATE_DIRECTORIES {
            assert!(
                files
                    .iter()
                    .any(|file| file.relative().starts_with(&format!("{directory}/"))),
                "no source under {directory}"
            );
        }
    }

    #[test]
    fn parsed_sources_know_their_crate_directory_relative_to_the_root() {
        let parsed = Repository::sources()
            .parsed()
            .expect("the repository reads");
        for directory in Repository::CRATE_DIRECTORIES {
            assert!(
                parsed
                    .iter()
                    .any(|source| source.crate_directory() == directory),
                "no source in {directory}"
            );
        }
    }
}
