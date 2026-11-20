use std::path::{Path, PathBuf};

use crate::cli::compiler_check::CompilerCheck;
use crate::cli::exit_status::ExitStatus;
use crate::cli::located_project::ProjectRoot;
use crate::cli::program_file::ProgramFile;
use crate::cli::terminal::Terminal;
use crate::cli::working_directory::WorkingDirectory;

pub struct ComparedRoots {
    roots: Vec<ProjectRoot>,
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
        let check = CompilerCheck::new(terminal);
        for root in &self.roots {
            check.refuse_before_loading(root)?;
        }
        for root in &self.roots {
            check.report(root)?;
        }
        Ok(())
    }

    fn root_around(path: &Path) -> Option<ProjectRoot> {
        let start = if path.is_dir() {
            path.to_path_buf()
        } else {
            ProgramFile::directory_of(path)
        };
        ProjectRoot::around(&WorkingDirectory::path().join(start))
    }
}
