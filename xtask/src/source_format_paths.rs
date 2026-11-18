use std::path::Path;

use crate::failure::TaskFailure;
use crate::git::Git;

pub struct SourceFormatPaths {
    checked: Vec<&'static str>,
    skipped: Vec<&'static str>,
}

impl SourceFormatPaths {
    const LIST: &'static str = include_str!("../source-format-paths.txt");
    const CHECK: &'static str = "check ";
    const SKIP: &'static str = "skip ";
    const DIRECTORY_SEPARATOR: char = '/';
    const LIST_TRACKED_FILES: &'static str = "ls-files";
    const END_OF_OPTIONS: &'static str = "--";

    fn parse(list: &'static str) -> Self {
        Self {
            checked: Self::paths_after(list, Self::CHECK),
            skipped: Self::paths_after(list, Self::SKIP),
        }
    }

    fn paths_after(list: &'static str, verb: &str) -> Vec<&'static str> {
        list.lines()
            .filter_map(|line| line.strip_prefix(verb))
            .collect()
    }

    pub fn files(&self, root: &Path, extension: &str) -> Result<Vec<String>, TaskFailure> {
        let suffix = format!(".{extension}");
        Ok(self
            .tracked_files(root)?
            .lines()
            .filter(|file| file.ends_with(&suffix) && self.checks(file))
            .filter(|file| root.join(file).is_file())
            .map(String::from)
            .collect())
    }

    fn tracked_files(&self, root: &Path) -> Result<String, TaskFailure> {
        let mut arguments = vec![Self::LIST_TRACKED_FILES, Self::END_OF_OPTIONS];
        arguments.extend(&self.checked);
        let output = Git::new(root).run(&arguments)?;
        if !output.status.success() {
            return Err(TaskFailure::tool_failed("git ls-files", &output.stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn checks(&self, file: &str) -> bool {
        Self::under_any(&self.checked, file) && !Self::under_any(&self.skipped, file)
    }

    fn under_any(directories: &[&str], file: &str) -> bool {
        directories.iter().any(|directory| {
            file.strip_prefix(directory)
                .is_some_and(|rest| rest.starts_with(Self::DIRECTORY_SEPARATOR))
        })
    }
}

impl Default for SourceFormatPaths {
    fn default() -> Self {
        Self::parse(Self::LIST)
    }
}

#[cfg(test)]
mod tests {
    use super::SourceFormatPaths;
    use crate::temporary_repository::TemporaryRepository;

    struct PathsCase;

    impl PathsCase {
        const LIST: &str = "check prelude\ncheck tests/fixtures\nskip tests/fixtures/read\n";

        fn checks(file: &str) -> bool {
            SourceFormatPaths::parse(Self::LIST).checks(file)
        }
    }

    struct TreeCase;

    impl TreeCase {
        fn files(repository: &TemporaryRepository) -> Result<Vec<String>, String> {
            SourceFormatPaths::parse("check prelude\n")
                .files(repository.root(), "knv")
                .map_err(|failure| failure.to_string())
        }
    }

    #[test]
    fn a_tree_outside_any_repository_fails() {
        let repository = TemporaryRepository::outside_git("paths-outside");
        repository.write("prelude/a.knv", "");
        assert!(TreeCase::files(&repository).is_err());
    }

    #[test]
    fn only_tracked_files_that_still_exist_are_listed() {
        let repository = TemporaryRepository::initialized("paths-tracked");
        for file in ["prelude/kept.knv", "prelude/deleted.knv"] {
            repository.write(file, "");
        }
        repository.git(&["add", "."]);
        repository.write("prelude/untracked.knv", "");
        repository.remove("prelude/deleted.knv");
        assert_eq!(
            TreeCase::files(&repository),
            Ok(vec!["prelude/kept.knv".to_string()])
        );
    }

    #[test]
    fn a_file_under_a_checked_directory_is_checked() {
        assert!(PathsCase::checks("prelude/core.knv"));
        assert!(PathsCase::checks("tests/fixtures/run/loop.knv"));
    }

    #[test]
    fn a_file_under_a_skipped_directory_is_not_checked() {
        assert!(!PathsCase::checks("tests/fixtures/read/bad.knv"));
    }

    #[test]
    fn a_file_outside_every_checked_directory_is_not_checked() {
        assert!(!PathsCase::checks("src/core.knv"));
        assert!(!PathsCase::checks("preludes/core.knv"));
    }

    #[test]
    fn the_files_are_the_checked_sources_of_the_repository() {
        let root = crate::workspace::WorkspaceFixture::repository_root();
        let files = SourceFormatPaths::default()
            .files(root, "knv")
            .expect("the tree reads");
        assert!(files.contains(&"examples/hello.knv".to_string()));
        assert!(files.iter().all(|file| file.ends_with(".knv")));
        assert!(
            !files
                .iter()
                .any(|file| file.starts_with("tests/fixtures/read/"))
        );
    }

    #[test]
    fn the_listed_paths_are_the_ones_the_flake_check_reads() {
        let paths = SourceFormatPaths::default();
        assert!(paths.checks("examples/hello.knv"));
        assert!(!paths.checks("tests/fixtures/expand/unreadable/a.knv"));
    }
}
