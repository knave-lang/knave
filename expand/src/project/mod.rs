mod dependency_reader;
mod fields;
mod name;
mod problems;
mod reader;

use std::path::PathBuf;

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

#[derive(Debug)]
pub struct CrateDependency {
    pub name: String,
    pub version: Option<String>,
    pub features: Vec<String>,
    pub path: Option<PathBuf>,
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
