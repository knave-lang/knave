mod crate_set;
mod dependency_reader;
mod fields;
mod name;
mod problems;
mod reader;

use std::path::PathBuf;

pub use crate_set::CrateSet;
pub use dependency_reader::Section;
pub use name::ProjectName;
pub use problems::ProjectProblems;
pub use reader::ProjectReader;

use crate::diagnostic::Diagnostics;
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
    pub sections: Vec<SectionHead>,
}

#[derive(Debug)]
pub struct SectionHead {
    pub section: Section,
    pub span: Span,
    pub has_entries: bool,
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

impl Project {
    pub fn unbuilt_sections(&self) -> Diagnostics {
        let mut problems = Diagnostics::default();
        self.sections
            .iter()
            .filter(|head| head.has_entries && !head.section.is_built())
            .for_each(|head| {
                problems.push(ProjectProblems::unbuilt_section(
                    head.section.name(),
                    head.span,
                ))
            });
        problems
    }
}
