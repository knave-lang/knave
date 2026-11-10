use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub enum SearchScope {
    #[default]
    Program,
    InProject {
        tasks: Option<PathBuf>,
    },
    Tests {
        tasks: Option<PathBuf>,
    },
    Tasks {
        tasks: PathBuf,
        tests: Option<PathBuf>,
    },
}

impl SearchScope {
    pub const TASKS_DIRECTORY: &'static str = "tasks";
    pub const TESTS_DIRECTORY: &'static str = "tests";
    const TEST_TREE: &'static str = "test";

    pub fn in_project(tasks: &Path) -> Self {
        Self::InProject {
            tasks: Self::canonical(tasks),
        }
    }

    pub fn tests(tasks: &Path) -> Self {
        Self::Tests {
            tasks: Self::canonical(tasks),
        }
    }

    pub fn tasks(tasks: &Path, tests: &Path) -> Self {
        Self::Tasks {
            tasks: Self::canonical(tasks).unwrap_or_else(|| tasks.to_path_buf()),
            tests: Self::canonical(tests),
        }
    }

    pub fn primary_reaches(&self, path: &Path) -> bool {
        match self {
            Self::Tasks { tests, .. } => !Self::lies_under(path, tests.as_deref()),
            _ => true,
        }
    }

    pub fn fallback_reaches(&self, path: &Path) -> bool {
        match self {
            Self::Tasks { tasks, .. } => {
                self.primary_reaches(path) && !Self::lies_under(path, Some(tasks))
            }
            _ => true,
        }
    }

    pub fn holds_task(&self, file: &Path) -> bool {
        Self::lies_under(file, self.tasks_directory())
    }

    pub fn directory_name(&self) -> &'static str {
        match self {
            Self::Tasks { .. } => Self::TASKS_DIRECTORY,
            _ => Self::TEST_TREE,
        }
    }

    fn tasks_directory(&self) -> Option<&Path> {
        match self {
            Self::Program => None,
            Self::InProject { tasks } | Self::Tests { tasks } => tasks.as_deref(),
            Self::Tasks { tasks, .. } => Some(tasks),
        }
    }

    fn lies_under(path: &Path, directory: Option<&Path>) -> bool {
        let Some(directory) = directory else {
            return false;
        };
        fs::canonicalize(path).is_ok_and(|path| path.starts_with(directory))
    }

    fn canonical(directory: &Path) -> Option<PathBuf> {
        fs::canonicalize(directory).ok()
    }
}
