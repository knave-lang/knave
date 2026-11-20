use std::path::{Path, PathBuf};

use crate::cli::compiler_check::CompilerCheck;
use crate::cli::exit_status::ExitStatus;
use crate::cli::located_project::ProjectRoot;
use crate::cli::program_file::ProgramFile;
use crate::cli::terminal::Terminal;
use crate::cli::working_directory::WorkingDirectory;
use crate::language::Language;

pub struct ComparedRoots {
    roots: Vec<PathBuf>,
}

impl ComparedRoots {
    pub fn around(paths: &[PathBuf]) -> Self {
        let mut roots = Vec::new();
        for root in paths.iter().filter_map(|path| Self::root_around(path)) {
            if !roots.contains(&root) {
                roots.push(root);
            }
        }
        Self { roots }
    }

    pub fn compare_versions(&self, terminal: &Terminal) -> Result<(), ExitStatus> {
        for root in &self.roots {
            CompilerCheck::refuse_before_loading(root, terminal)?;
        }
        for root in &self.roots {
            CompilerCheck::EVERY_MISMATCH
                .report(&root.join(Language::project_file_name()), terminal)?;
        }
        Ok(())
    }

    fn root_around(path: &Path) -> Option<PathBuf> {
        let start = if path.is_dir() {
            path.to_path_buf()
        } else {
            ProgramFile::directory_of(path)
        };
        ProjectRoot::canonical_of(&WorkingDirectory::path().join(start))
    }
}
