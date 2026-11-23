mod compiler_version;
mod compiler_version_problems;
mod crate_set;
mod dependency_reader;
mod dependency_section;
mod exported_task_problems;
mod exported_task_reader;
mod fields;
mod level_keys;
mod level_words;
mod lint_level;
mod lint_level_problems;
mod lint_name_problems;
mod lint_reader;
mod lint_table;
mod name;
mod problems;
mod project_editor;
mod project_modules;
mod project_roles;
mod quoted_list;
mod reader;
mod release_version;
mod role_book;
mod role_note;
mod role_problem;
mod section;
mod section_names;
mod vocabulary;

use std::path::PathBuf;

pub use compiler_version::CompilerVersion;
pub use crate_set::CrateSet;
pub use dependency_section::DependencySection;
pub(crate) use level_keys::LevelKeys;
pub use lint_level::LintLevel;
pub use lint_table::LintTable;
pub use name::ProjectName;
pub use problems::ProjectProblems;
pub use project_editor::{CrateEntry, ProjectEditor};
pub use project_modules::ProjectModules;
pub(crate) use project_roles::ProjectRoles;
pub use reader::ProjectReader;
pub use release_version::ReleaseVersion;
pub use section::Section;
pub use vocabulary::ProjectVocabulary;

use crate::source::Span;

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub version: String,
    pub modules: ProjectModules,
    pub edition: i128,
    pub compiler_version: Option<CompilerVersion>,
    pub libraries: Vec<LibraryDependency>,
    pub dependencies: Vec<CrateDependency>,
    pub dev_dependencies: Vec<CrateDependency>,
    pub lints: LintTable,
    pub exported_tasks: Vec<ExportedTask>,
}

#[derive(Debug)]
pub struct ExportedTask {
    pub name: String,
    pub span: Span,
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
