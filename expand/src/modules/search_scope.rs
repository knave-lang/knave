use std::fs;
use std::path::{Component, Path, PathBuf};

use super::ProjectFolder;
use super::lexical_path::LexicalPath;
use crate::language::Language;

#[derive(Clone, Debug, Default)]
pub enum SearchScope {
    #[default]
    Program,
    Project {
        tree: ProjectTree,
        kind: ProjectScope,
    },
}

#[derive(Clone, Debug)]
pub enum ProjectScope {
    Sources,
    Tests,
    Tasks { tasks: PathBuf },
}

impl SearchScope {
    pub const TASKS_DIRECTORY: &'static str = ProjectFolder::Tasks.name();
    pub const TESTS_DIRECTORY: &'static str = ProjectFolder::Tests.name();
    const TEST_TREE: &'static str = "test";

    pub fn in_project(root: &Path) -> Self {
        Self::within(ProjectTree::new(root), ProjectScope::Sources)
    }

    pub fn tests(root: &Path) -> Self {
        Self::within(ProjectTree::new(root), ProjectScope::Tests)
    }

    pub fn tasks(root: &Path) -> Self {
        let tree = ProjectTree::new(root);
        let tasks = tree.tasks_or_shown();
        Self::within(tree, ProjectScope::Tasks { tasks })
    }

    pub fn primary_reaches(&self, path: &Path) -> bool {
        match self {
            Self::Project {
                tree,
                kind: ProjectScope::Tasks { .. },
            } => !ProjectTree::lies_under(path, tree.tests.as_deref()),
            _ => true,
        }
    }

    pub fn fallback_reaches(&self, path: &Path) -> bool {
        match self {
            Self::Project {
                kind: ProjectScope::Tasks { tasks },
                ..
            } => self.primary_reaches(path) && !ProjectTree::lies_under(path, Some(tasks)),
            _ => true,
        }
    }

    pub fn project_folders(&self, file: &Path) -> Vec<ProjectFolder> {
        match self {
            Self::Program => Vec::new(),
            Self::Project { tree, .. } => tree.folders_of(file),
        }
    }

    pub fn directory_name(&self) -> &'static str {
        match self {
            Self::Project {
                kind: ProjectScope::Tasks { .. },
                ..
            } => Self::TASKS_DIRECTORY,
            _ => Self::TEST_TREE,
        }
    }

    fn within(tree: ProjectTree, kind: ProjectScope) -> Self {
        Self::Project { tree, kind }
    }
}

#[derive(Clone, Debug)]
pub struct ProjectTree {
    shown_root: LexicalPath,
    root: Option<PathBuf>,
    tasks: Option<PathBuf>,
    tests: Option<PathBuf>,
}

impl ProjectTree {
    const WORKING_DIRECTORY: &'static str = ".";

    pub fn holds_a_project(directory: &Path) -> bool {
        directory.join(Language::project_file_name()).is_file()
    }

    fn new(shown_root: &Path) -> Self {
        let root = Path::new(Self::WORKING_DIRECTORY).join(shown_root);
        Self {
            shown_root: LexicalPath::absolute(&root),
            root: Self::canonical(&root),
            tasks: Self::canonical(&root.join(SearchScope::TASKS_DIRECTORY)),
            tests: Self::canonical(&root.join(SearchScope::TESTS_DIRECTORY)),
        }
    }

    fn tasks_or_shown(&self) -> PathBuf {
        self.tasks
            .clone()
            .unwrap_or_else(|| self.shown_root.as_path().join(SearchScope::TASKS_DIRECTORY))
    }

    fn folders_of(&self, file: &Path) -> Vec<ProjectFolder> {
        let tasks = self
            .followed_into_tasks(file)
            .then_some(ProjectFolder::Tasks);
        let loaded = self.loaded_folder(&LexicalPath::absolute(file));
        tasks.into_iter().chain(loaded).collect()
    }

    fn followed_into_tasks(&self, file: &Path) -> bool {
        Self::lies_under(file, self.tasks.as_deref())
    }

    fn loaded_folder(&self, loaded: &LexicalPath) -> Option<ProjectFolder> {
        let named = loaded.as_path();
        match self.root_holding(named) {
            Some(root) => Self::folder_under(named, root),
            None => self.real_folder(named),
        }
    }

    fn real_folder(&self, named: &Path) -> Option<ProjectFolder> {
        let root = self.root.as_deref()?;
        let real = Self::canonical(named)?;
        Self::folder_under(&real, root)
    }

    fn folder_under(loaded: &Path, root: &Path) -> Option<ProjectFolder> {
        match loaded.strip_prefix(root).ok()?.components().next() {
            Some(Component::Normal(name)) if name == SearchScope::TESTS_DIRECTORY => {
                Some(ProjectFolder::Tests)
            }
            _ => (!Self::lies_in_nested_project(loaded, root)).then_some(ProjectFolder::Source),
        }
    }

    fn root_holding(&self, loaded: &Path) -> Option<&Path> {
        [Some(self.shown_root.as_path()), self.root.as_deref()]
            .into_iter()
            .flatten()
            .find(|root| loaded.starts_with(root))
    }

    fn lies_in_nested_project(loaded: &Path, root: &Path) -> bool {
        loaded
            .ancestors()
            .skip(1)
            .take_while(|folder| *folder != root)
            .any(Self::holds_a_project)
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::SearchScope;
    use crate::language::Language;
    use crate::modules::ProjectFolder::{self, Source, Tasks, Tests};

    struct ProjectFixture {
        folder: PathBuf,
        root: PathBuf,
    }

    impl ProjectFixture {
        const FILES: [&str; 9] = [
            "src/main.knv",
            "shared/outside.knv",
            "tasks/count.knv",
            "tasks/lib/project.knv",
            "tasks/lib/src/lib.knv",
            "tests/deep/counting.knv",
            "src/vendor/shelving/src/lib.knv",
            "src/vendor/shelving/project.knv",
            "project.knv",
        ];

        const REAL_ROOT: &str = "real";
        const SHOWN_ROOT: &str = "shown";

        fn new(name: &str) -> Self {
            let folder = Self::folder(name);
            Self::write_files(&folder);
            Self {
                root: folder.clone(),
                folder,
            }
        }

        #[cfg(unix)]
        fn reached_through_link(name: &str) -> Self {
            let folder = Self::folder(name);
            Self::write_files(&folder.join(Self::REAL_ROOT));
            let root = folder.join(Self::SHOWN_ROOT);
            std::os::unix::fs::symlink(Self::REAL_ROOT, &root).expect("the link is writable");
            Self { folder, root }
        }

        fn folder(name: &str) -> PathBuf {
            std::env::temp_dir()
                .join(format!(
                    "{}-search-scope-{}",
                    Language::NAME,
                    std::process::id()
                ))
                .join(name)
        }

        fn write_files(root: &Path) {
            for file in Self::FILES {
                let path = root.join(file);
                fs::create_dir_all(path.parent().expect("a fixture file has a folder"))
                    .expect("the fixture folder is writable");
                fs::write(path, "").expect("the fixture file is writable");
            }
        }

        fn folders(&self, scope: &SearchScope, file: &str) -> Vec<ProjectFolder> {
            scope.project_folders(&self.path(file))
        }

        fn path(&self, file: &str) -> PathBuf {
            self.root.join(file)
        }

        #[cfg(unix)]
        fn link(&self, link: &str, target: &str) {
            std::os::unix::fs::symlink(target, self.path(link)).expect("the link is writable");
        }
    }

    impl Drop for ProjectFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.folder);
        }
    }

    #[test]
    fn names_a_task_file_as_tasks_and_source() {
        let fixture = ProjectFixture::new("task-file");
        let scope = SearchScope::in_project(&fixture.root);
        assert_eq!(fixture.folders(&scope, "tasks/count.knv"), [Tasks, Source]);
    }

    #[test]
    fn names_a_library_under_tasks_as_tasks_alone() {
        let fixture = ProjectFixture::new("task-library");
        let scope = SearchScope::tasks(&fixture.root);
        assert_eq!(fixture.folders(&scope, "tasks/lib/src/lib.knv"), [Tasks]);
    }

    #[test]
    fn names_a_test_file_as_tests() {
        let fixture = ProjectFixture::new("test-file");
        let scope = SearchScope::tests(&fixture.root);
        assert_eq!(fixture.folders(&scope, "tests/deep/counting.knv"), [Tests]);
    }

    #[test]
    fn names_a_source_file_as_source() {
        let fixture = ProjectFixture::new("source-file");
        let scope = SearchScope::in_project(&fixture.root);
        assert_eq!(fixture.folders(&scope, "src/main.knv"), [Source]);
    }

    #[test]
    fn names_no_folder_for_a_nested_project_or_a_file_outside_the_root() {
        let fixture = ProjectFixture::new("outside");
        let scope = SearchScope::tests(&fixture.path("src"));
        assert!(
            fixture
                .folders(&scope, "src/vendor/shelving/src/lib.knv")
                .is_empty()
        );
        assert!(fixture.folders(&scope, "tasks/count.knv").is_empty());
        assert!(
            fixture
                .folders(&SearchScope::Program, "src/main.knv")
                .is_empty()
        );
    }

    #[test]
    fn a_task_scope_falls_back_only_outside_tasks_and_tests() {
        let fixture = ProjectFixture::new("tasks");
        let scope = SearchScope::tasks(&fixture.root);
        assert!(scope.fallback_reaches(&fixture.path("src/main.knv")));
        assert!(!scope.fallback_reaches(&fixture.path("tasks/count.knv")));
        assert!(!scope.primary_reaches(&fixture.path("tests/deep/counting.knv")));
        assert!(!scope.fallback_reaches(&fixture.path("tests/deep/counting.knv")));
    }

    #[test]
    fn a_shown_root_of_the_working_directory_reads_as_the_working_directory() {
        let scope = SearchScope::in_project(Path::new(""));
        assert_eq!(scope.project_folders(Path::new("src/lib.rs")), [Source]);
    }

    #[cfg(unix)]
    #[test]
    fn a_root_shown_with_a_parent_step_through_a_link_holds_its_tests_and_sources() {
        let fixture = ProjectFixture::new("parent-step");
        fixture.link("linked", ".");
        let scope = SearchScope::tests(&fixture.path("linked/src/.."));
        assert_eq!(
            fixture.folders(&scope, "linked/tests/deep/counting.knv"),
            [Tests]
        );
        assert_eq!(fixture.folders(&scope, "linked/src/main.knv"), [Source]);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_named_under_the_real_folder_of_a_linked_root_holds_its_folder() {
        let fixture = ProjectFixture::new("linked-root");
        fixture.link("linked", ".");
        let scope = SearchScope::tests(&fixture.path("linked"));
        assert_eq!(fixture.folders(&scope, "src/main.knv"), [Source]);
        assert_eq!(fixture.folders(&scope, "tests/deep/counting.knv"), [Tests]);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_named_under_a_project_reached_through_a_link_holds_its_folder() {
        let fixture = ProjectFixture::reached_through_link("root-through-link");
        fixture.link("linked", ".");
        let scope = SearchScope::tests(&fixture.path("linked"));
        assert_eq!(fixture.folders(&scope, "src/main.knv"), [Source]);
        assert_eq!(fixture.folders(&scope, "tests/deep/counting.knv"), [Tests]);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_test_file_or_folder_sits_where_the_tests_tree_loads_it() {
        let fixture = ProjectFixture::new("linked-tests");
        fixture.link("tests/aliased.knv", "../src/main.knv");
        fixture.link("tests/shared", "../shared");
        fixture.link("src/borrowed.knv", "../tests/deep/counting.knv");
        let scope = SearchScope::tests(&fixture.root);
        assert_eq!(fixture.folders(&scope, "tests/aliased.knv"), [Tests]);
        assert_eq!(fixture.folders(&scope, "tests/shared/outside.knv"), [Tests]);
        assert_eq!(fixture.folders(&scope, "src/borrowed.knv"), [Source]);
    }

    #[cfg(unix)]
    #[test]
    fn a_test_file_linked_into_tasks_is_in_tasks_and_tests() {
        let fixture = ProjectFixture::new("test-linked-into-tasks");
        fixture.link("tests/tasked.knv", "../tasks/count.knv");
        let scope = SearchScope::tests(&fixture.root);
        assert_eq!(fixture.folders(&scope, "tests/tasked.knv"), [Tasks, Tests]);
    }

    #[cfg(unix)]
    #[test]
    fn a_task_file_is_in_tasks_where_its_link_points() {
        let fixture = ProjectFixture::new("linked-tasks");
        fixture.link("tasks/borrowed.knv", "../src/main.knv");
        fixture.link("src/tasked.knv", "../tasks/count.knv");
        let scope = SearchScope::tasks(&fixture.root);
        assert_eq!(fixture.folders(&scope, "tasks/borrowed.knv"), [Source]);
        assert_eq!(fixture.folders(&scope, "src/tasked.knv"), [Tasks, Source]);
    }
}
