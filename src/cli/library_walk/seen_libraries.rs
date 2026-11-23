use std::path::{Path, PathBuf};

use crate::cli::program_file::ProgramFile;
use crate::diagnostic::Diagnostic;
use crate::project::{LibraryDependency, Project, ProjectProblems};
use crate::source::Span;

#[derive(Default)]
pub struct SeenLibraries {
    seen: Vec<SeenLibrary>,
}

struct SeenLibrary {
    name: String,
    root: PathBuf,
    span: Span,
    exported_tasks: Vec<String>,
}

impl SeenLibraries {
    pub fn holds(&self, library: &LibraryDependency, root: &Path) -> Result<bool, Diagnostic> {
        let Some(seen) = self.seen.iter().find(|seen| seen.name == library.name) else {
            return Ok(false);
        };
        if seen.root == Self::identity(root) {
            return Ok(true);
        }
        Err(ProjectProblems::library_at_two_sources(
            &library.name,
            library.name_span,
            seen.span,
        ))
    }

    pub fn add(&mut self, root: &Path, library: &LibraryDependency, project: &Project) {
        self.seen.push(SeenLibrary {
            name: library.name.clone(),
            root: Self::identity(root),
            span: library.name_span,
            exported_tasks: project.exported_tasks.names(),
        });
    }

    pub fn exported_tasks_at(&self, root: &Path) -> Vec<String> {
        let identity = Self::identity(root);
        self.seen
            .iter()
            .find(|seen| seen.root == identity)
            .map(|seen| seen.exported_tasks.clone())
            .unwrap_or_default()
    }

    fn identity(root: &Path) -> PathBuf {
        ProgramFile::canonical(root).unwrap_or(root.to_path_buf())
    }
}
