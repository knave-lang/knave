use std::path::Path;
use std::process::Output;

use crate::cargo::Cargo;
use crate::checks::Check;
use crate::failure::TaskFailure;
use crate::language::LanguageSource;
use crate::source_format_paths::SourceFormatPaths;
use crate::workspace::CargoWorkspace;

pub struct SourceFormat {
    checked_files: usize,
    output: Output,
}

impl SourceFormat {
    const FORMAT_CHECK: [&str; 5] = ["run", "--quiet", "--", "fmt", "--check"];

    pub fn check(root: &Path) -> Result<Self, TaskFailure> {
        let workspace = CargoWorkspace::read(root)?;
        let extension = LanguageSource::find(&workspace)?.extension()?;
        let files = SourceFormatPaths::default().files(root, &extension)?;
        let mut command = Cargo::command();
        command
            .current_dir(root)
            .args(Self::FORMAT_CHECK)
            .args(&files);
        let output = command
            .output()
            .map_err(|error| TaskFailure::tool_missing("cargo", error))?;
        Ok(Self::finished(files.len(), output))
    }

    pub fn finished(checked_files: usize, output: Output) -> Self {
        Self {
            checked_files,
            output,
        }
    }
}

impl Check for SourceFormat {
    fn passes(&self) -> bool {
        self.output.status.success()
    }

    fn summary(&self) -> String {
        format!(
            "{} source files laid out by `knave fmt --check`, the files the `source-format` flake check reads",
            self.checked_files
        )
    }

    fn details(&self) -> String {
        format!(
            "{}{}Lay out each file with `cargo run --quiet -- fmt <file>`.\n",
            String::from_utf8_lossy(&self.output.stdout),
            String::from_utf8_lossy(&self.output.stderr)
        )
    }
}

#[cfg(test)]
pub struct FormatOutputs;

#[cfg(test)]
impl FormatOutputs {
    const FAILED_STATUS: i32 = 1 << 8;

    pub fn with_status(status: i32) -> Output {
        use std::os::unix::process::ExitStatusExt;
        Output {
            status: std::process::ExitStatus::from_raw(status),
            stdout: b"prelude/core.knv\n".to_vec(),
            stderr: Vec::new(),
        }
    }

    pub fn failed() -> Output {
        Self::with_status(Self::FAILED_STATUS)
    }

    pub fn passed() -> Output {
        Self::with_status(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{FormatOutputs, SourceFormat};
    use crate::checks::Check;

    #[test]
    fn a_failed_format_check_fails_and_names_its_files() {
        let check = SourceFormat::finished(1, FormatOutputs::failed());
        assert!(!check.passes());
        assert_eq!(
            check.details(),
            "prelude/core.knv\nLay out each file with `cargo run --quiet -- fmt <file>`.\n"
        );
    }

    #[test]
    fn a_passed_format_check_passes_and_counts_its_files() {
        let check = SourceFormat::finished(2633, FormatOutputs::passed());
        assert!(check.passes());
        assert!(
            check.summary().starts_with("2633 source files"),
            "{}",
            check.summary()
        );
    }
}
