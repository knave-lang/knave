mod crate_set;
mod dependency_reader;
mod fields;
mod name;
mod problems;
mod project_editor;
mod reader;

use std::path::PathBuf;

pub use crate_set::CrateSet;
pub use dependency_reader::Section;
pub use name::ProjectName;
pub use problems::ProjectProblems;
pub use project_editor::{CrateEntry, ProjectEditor};
pub use reader::ProjectReader;

use crate::source::Span;

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub version: String,
    pub entry: PathBuf,
    pub library_root: PathBuf,
    pub edition: i128,
    pub libraries: Vec<LibraryDependency>,
    pub dependencies: Vec<CrateDependency>,
    pub dev_dependencies: Vec<CrateDependency>,
}

#[derive(Debug)]
pub struct LibraryDependency {
    pub name: String,
    pub name_span: Span,
    pub source: LibrarySource,
}

#[derive(Debug)]
pub enum LibrarySource {
    Path(PathBuf),
    Git { url: String, commit: String },
}

impl LibrarySource {
    pub const FULL_COMMIT_LENGTH: usize = 40;
}

#[derive(Clone, Debug)]
pub struct CrateDependency {
    pub name: String,
    pub name_span: Span,
    pub version: Option<String>,
    pub features: Vec<String>,
    pub path: Option<PathBuf>,
}

impl CrateDependency {
    pub fn same_request(&self, other: &Self) -> bool {
        self.version == other.version
            && self.path == other.path
            && Self::covers(&self.features, &other.features)
            && Self::covers(&other.features, &self.features)
    }

    fn covers(features: &[String], others: &[String]) -> bool {
        others.iter().all(|feature| features.contains(feature))
    }

    pub fn resolved_from(&self, root: &std::path::Path) -> Self {
        Self {
            path: self.path.as_ref().map(|path| root.join(path)),
            ..self.clone()
        }
    }
}
