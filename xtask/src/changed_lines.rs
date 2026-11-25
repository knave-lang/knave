use std::collections::BTreeMap;
use std::path::Path;

use crate::failure::TaskFailure;
use crate::fork_point::ForkPoint;
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

    pub fn since(root: &Path, fork_point: &ForkPoint) -> Result<Self, TaskFailure> {
        let output = Git::new(root).run(&[
            "diff",
            "-U0",
            "--no-color",
            "--no-ext-diff",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            fork_point.as_str(),
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
    use crate::fork_point::ForkPoint;
    use crate::functions::LineSpan;
    use crate::git::Git;
    use crate::temporary_repository::TemporaryRepository;

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

    #[test]
    fn a_change_shows_under_a_user_config_without_path_prefixes() {
        let repository = TemporaryRepository::initialized("noprefix");
        repository.write("src/a.rs", "fn a() {\n    1\n}\n");
        repository.commit_all();
        repository.git(&["config", "diff.noprefix", "true"]);
        repository.write("src/a.rs", "fn a() {\n    2\n}\n");
        let head = ForkPoint::resolve(&Git::new(repository.root()), "HEAD").expect("HEAD resolves");
        let changed = ChangedLines::since(repository.root(), &head).expect("git diff runs");
        assert!(changed.touches("src/a.rs", LineSpan { first: 2, last: 2 }));
    }

    #[test]
    fn a_failed_diff_is_a_failure() {
        let root = crate::workspace::WorkspaceFixture::repository_root();
        let head = ForkPoint::resolve(&Git::new(root), "HEAD").expect("HEAD resolves");
        let outside = TemporaryRepository::outside_git("failed-diff");
        assert!(ChangedLines::since(outside.root(), &head).is_err());
    }

    #[test]
    fn a_malformed_hunk_header_changes_nothing() {
        let changed = ChangedLines::parse("+++ b/src/a.rs\n@@ -1 +x @@\n@@ -1 +2,y @@\n");
        assert!(!changed.touches("src/a.rs", LineSpan { first: 1, last: 99 }));
    }
}
