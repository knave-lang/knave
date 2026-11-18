use std::collections::BTreeMap;
use std::path::Path;

use crate::failure::TaskFailure;
use crate::functions::LineSpan;
use crate::git::Git;

pub struct ChangedLines {
    spans: BTreeMap<String, Vec<LineSpan>>,
}

impl ChangedLines {
    const FILE_HEADER: &str = "+++ ";
    const NEW_SIDE_PREFIX: &str = "b/";
    const HUNK_HEADER: &str = "@@ ";
    const NEW_SIDE_RANGE: char = '+';
    const RANGE_SEPARATOR: char = ',';

    pub fn since(root: &Path, base: &str) -> Result<Self, TaskFailure> {
        let output = Git::new(root).run(&[
            "diff",
            "-U0",
            "--no-color",
            "--no-ext-diff",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            base,
        ])?;
        if !output.status.success() {
            return Err(TaskFailure::tool_failed("git diff", &output.stderr));
        }
        Ok(Self::parse(&String::from_utf8_lossy(&output.stdout)))
    }

    pub fn parse(diff: &str) -> Self {
        let mut spans: BTreeMap<String, Vec<LineSpan>> = BTreeMap::new();
        let mut file = None;
        for line in diff.lines() {
            if let Some(header) = line.strip_prefix(Self::FILE_HEADER) {
                file = header.strip_prefix(Self::NEW_SIDE_PREFIX).map(String::from);
            } else if let (Some(file), Some(span)) = (&file, Self::hunk_span(line)) {
                spans.entry(file.clone()).or_default().push(span);
            }
        }
        Self { spans }
    }

    fn hunk_span(line: &str) -> Option<LineSpan> {
        let range = line
            .strip_prefix(Self::HUNK_HEADER)?
            .split_whitespace()
            .find_map(|word| word.strip_prefix(Self::NEW_SIDE_RANGE))?;
        let (first, count) = match range.split_once(Self::RANGE_SEPARATOR) {
            Some((first, count)) => (first.parse().ok()?, count.parse().ok()?),
            None => (range.parse().ok()?, 1),
        };
        Some(Self::span_of(first, count))
    }

    fn span_of(first: usize, count: usize) -> LineSpan {
        match count {
            0 => LineSpan {
                first,
                last: first + 1,
            },
            _ => LineSpan {
                first,
                last: first + count - 1,
            },
        }
    }

    pub fn touches(&self, file: &str, span: LineSpan) -> bool {
        self.spans
            .get(file)
            .is_some_and(|spans| spans.iter().any(|changed| changed.overlaps(span)))
    }
}

#[cfg(test)]
mod tests {
    use super::ChangedLines;
    use crate::functions::LineSpan;

    struct DiffCase;

    impl DiffCase {
        const DIFF: &str = "diff --git a/src/a.rs b/src/a.rs\n\
            --- a/src/a.rs\n\
            +++ b/src/a.rs\n\
            @@ -10,2 +10,3 @@ fn area() {\n\
            +    let width = 2;\n\
            @@ -30 +31 @@ fn depth() {\n\
            -    1\n\
            +    2\n\
            @@ -50,4 +51,0 @@ fn height() {\n\
            -    old\n\
            diff --git a/src/gone.rs b/src/gone.rs\n\
            --- a/src/gone.rs\n\
            +++ /dev/null\n\
            @@ -1,3 +0,0 @@\n\
            -fn gone() {}\n";

        fn touches(file: &str, first: usize, last: usize) -> bool {
            ChangedLines::parse(Self::DIFF).touches(file, LineSpan { first, last })
        }
    }

    #[test]
    fn a_span_that_holds_added_lines_is_touched() {
        assert!(DiffCase::touches("src/a.rs", 1, 10));
        assert!(DiffCase::touches("src/a.rs", 12, 20));
    }

    #[test]
    fn a_span_between_hunks_is_untouched() {
        assert!(!DiffCase::touches("src/a.rs", 13, 30));
    }

    #[test]
    fn a_hunk_without_a_count_changes_one_line() {
        assert!(DiffCase::touches("src/a.rs", 31, 31));
        assert!(!DiffCase::touches("src/a.rs", 32, 50));
    }

    #[test]
    fn a_deletion_touches_the_lines_on_both_sides_of_it() {
        assert!(DiffCase::touches("src/a.rs", 52, 60));
        assert!(DiffCase::touches("src/a.rs", 40, 51));
    }

    #[test]
    fn a_deleted_file_lists_no_lines() {
        assert!(!DiffCase::touches("src/gone.rs", 1, 3));
    }

    struct TemporaryRepository {
        root: std::path::PathBuf,
    }

    impl TemporaryRepository {
        const FILE: &str = "src/a.rs";

        fn with_changed_line(name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("xtask-diff-{name}-{}", std::process::id()));
            std::fs::create_dir_all(root.join("src")).expect("the repository is created");
            let repository = Self { root };
            repository.git(&["init", "--quiet"]);
            repository.write("fn a() {\n    1\n}\n");
            repository.git(&["add", "."]);
            repository.git(&[
                "-c",
                "user.name=fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "base",
            ]);
            repository.git(&["config", "diff.noprefix", "true"]);
            repository.write("fn a() {\n    2\n}\n");
            repository
        }

        fn write(&self, text: &str) {
            std::fs::write(self.root.join(Self::FILE), text).expect("the file is written");
        }

        fn git(&self, arguments: &[&str]) {
            let status = std::process::Command::new("git")
                .args(arguments)
                .current_dir(&self.root)
                .status()
                .expect("git runs");
            assert!(status.success(), "git {arguments:?} failed");
        }
    }

    impl Drop for TemporaryRepository {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn a_change_shows_under_a_user_config_without_path_prefixes() {
        let repository = TemporaryRepository::with_changed_line("noprefix");
        let changed = ChangedLines::since(&repository.root, "HEAD").expect("git diff runs");
        assert!(changed.touches(TemporaryRepository::FILE, LineSpan { first: 2, last: 2 }));
    }

    #[test]
    fn a_failed_diff_is_a_failure() {
        let root = crate::workspace::WorkspaceFixture::repository_root();
        assert!(ChangedLines::since(root, "no-such-revision").is_err());
    }

    #[test]
    fn a_malformed_hunk_header_changes_nothing() {
        let changed = ChangedLines::parse("+++ b/src/a.rs\n@@ -1 +x @@\n@@ -1 +2,y @@\n");
        assert!(!changed.touches("src/a.rs", LineSpan { first: 1, last: 99 }));
    }
}
