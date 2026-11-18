use std::path::Path;
use std::process::Output;

use crate::cargo::Cargo;
use crate::changed_lines::ChangedLines;
use crate::checks::Check;
use crate::failure::TaskFailure;
use crate::language::LanguageSource;
use crate::source_format_paths::SourceFormatPaths;
use crate::workspace::CargoWorkspace;

pub struct SourceFormat {
    files: Vec<String>,
    output: Option<Output>,
}

impl SourceFormat {
    const FORMAT_CHECK: [&str; 5] = ["run", "--quiet", "--", "fmt", "--check"];

    pub fn check(root: &Path, changed: &ChangedLines) -> Result<Self, TaskFailure> {
        let workspace = CargoWorkspace::read(root)?;
        let extension = LanguageSource::find(&workspace)?.extension()?;
        let files = Self::checked_files(changed, &format!(".{extension}"));
        let output = Self::run(root, &files)?;
        Ok(Self { files, output })
    }

    fn checked_files(changed: &ChangedLines, extension: &str) -> Vec<String> {
        let paths = SourceFormatPaths::default();
        changed
            .files()
            .filter(|file| file.ends_with(extension) && paths.checks(file))
            .map(String::from)
            .collect()
    }

    fn run(root: &Path, files: &[String]) -> Result<Option<Output>, TaskFailure> {
        if files.is_empty() {
            return Ok(None);
        }
        let mut command = Cargo::command();
        command
            .current_dir(root)
            .args(Self::FORMAT_CHECK)
            .args(files);
        command
            .output()
            .map(Some)
            .map_err(|error| TaskFailure::tool_missing("cargo", error))
    }
}

impl Check for SourceFormat {
    fn passes(&self) -> bool {
        self.output
            .as_ref()
            .is_none_or(|output| output.status.success())
    }

    fn summary(&self) -> String {
        format!(
            "{} changed source files checked as the `source-format` flake check does",
            self.files.len()
        )
    }

    fn details(&self) -> String {
        let Some(output) = &self.output else {
            return String::new();
        };
        format!(
            "{}{}Lay out each file with `cargo run --quiet -- fmt <file>`.\n",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::ExitStatusExt;
    use std::path::Path;
    use std::process::{ExitStatus, Output};

    use super::SourceFormat;
    use crate::changed_lines::ChangedLines;
    use crate::checks::Check;

    struct FormatCase;

    impl FormatCase {
        const FAILED_STATUS: i32 = 1 << 8;

        fn checked(files: &[&str]) -> Vec<String> {
            let diff: String = files
                .iter()
                .map(|file| format!("+++ b/{file}\n@@ -1 +1 @@\n"))
                .collect();
            SourceFormat::checked_files(&ChangedLines::parse(&diff), ".knv")
        }

        fn failed() -> SourceFormat {
            let output = Output {
                status: ExitStatus::from_raw(Self::FAILED_STATUS),
                stdout: b"prelude/core.knv\n".to_vec(),
                stderr: Vec::new(),
            };
            SourceFormat {
                files: vec!["prelude/core.knv".to_string()],
                output: Some(output),
            }
        }
    }

    #[test]
    fn only_sources_under_the_checked_directories_are_checked() {
        assert_eq!(
            FormatCase::checked(&[
                "prelude/core.knv",
                "examples/hello.knv",
                "tests/fixtures/run/loop.knv",
                "src/core.knv",
                "prelude/notes.md"
            ]),
            [
                "examples/hello.knv",
                "prelude/core.knv",
                "tests/fixtures/run/loop.knv"
            ]
        );
    }

    #[test]
    fn the_skipped_fixture_directories_are_not_checked() {
        assert!(
            FormatCase::checked(&[
                "tests/fixtures/read/a.knv",
                "tests/fixtures/format/b.knv",
                "tests/fixtures/expand/unreadable/c.knv"
            ])
            .is_empty()
        );
    }

    #[test]
    fn no_changed_sources_run_no_cargo_and_pass() {
        let output = SourceFormat::run(Path::new("/nonexistent/workspace"), &[]);
        assert!(output.expect("nothing runs").is_none());
        let check = SourceFormat {
            files: Vec::new(),
            output: None,
        };
        assert!(check.passes());
        assert_eq!(check.details(), "");
    }

    #[test]
    fn a_failed_format_check_fails_and_names_its_files() {
        let check = FormatCase::failed();
        assert!(!check.passes());
        assert_eq!(
            check.details(),
            "prelude/core.knv\nLay out each file with `cargo run --quiet -- fmt <file>`.\n"
        );
    }
}
