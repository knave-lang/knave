mod build_plan;
mod cargo_compiler;
mod cargo_manifest;
mod crate_resolver;
mod formatting;
mod git_store;
mod interrupt_signal;
mod profile;
mod rustc_errors;
mod token_alignment;
mod user_cache;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::diagnostic::Diagnostics;
use crate::emit::EmittedCrate;
use crate::source::Span;
pub use build_plan::BuildPlan;
pub use cargo_compiler::{CargoCompiler, CargoTarget};
pub use cargo_manifest::{CargoManifest, CargoTargetKind};
pub use crate_resolver::{CrateResolver, ResolveError, ResolvedCrate};
pub use formatting::{EmittedFormatting, FormattingError};
pub use git_store::{GitStore, StoreError};
pub use interrupt_signal::InterruptSignal;
pub use profile::BuildProfile;
use rustc_errors::RustcDiagnosticMapper;

pub enum BuildError {
    Program(Diagnostics),
    Environment(String),
    Interrupted,
}

pub struct RustCompiler {
    directory: PathBuf,
    profile: BuildProfile,
}

impl RustCompiler {
    const RUSTC: &str = "rustc";
    const SOURCE_FILE: &str = "main.rs";

    pub fn new(directory: PathBuf, profile: BuildProfile) -> Self {
        Self { directory, profile }
    }

    pub fn compile(
        &self,
        emitted: &EmittedCrate,
        crate_name: &str,
        output: &Path,
        entry: Span,
    ) -> Result<(), BuildError> {
        let source = self
            .write_source(emitted)
            .map_err(|error| Self::environment("write the generated Rust", &error))?;
        Self::create_output_directory(output)?;
        let result = self
            .rustc_command(crate_name, output, &source)
            .output()
            .map_err(|error| Self::environment("run rustc", &error))?;
        if result.status.success() {
            return Ok(());
        }
        if InterruptSignal::stopped(result.status) {
            return Err(BuildError::Interrupted);
        }
        let stderr = String::from_utf8_lossy(&result.stderr);
        Err(BuildError::Program(
            RustcDiagnosticMapper::new(emitted, entry).map(&stderr),
        ))
    }

    fn rustc_command(&self, crate_name: &str, output: &Path, source: &Path) -> Command {
        let mut command = Command::new(Self::RUSTC);
        command
            .args([
                "--edition",
                BuildPlan::EDITION,
                "--crate-name",
                crate_name,
                "--error-format=json",
            ])
            .args(BuildPlan::new(self.profile).lint_flags())
            .arg("-o")
            .arg(output)
            .arg(source)
            .args(BuildPlan::new(self.profile).rustc_flags());
        command
    }

    fn write_source(&self, emitted: &EmittedCrate) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.directory)?;
        let source = self.directory.join(Self::SOURCE_FILE);
        fs::write(&source, &emitted.code)?;
        Ok(source)
    }

    fn create_output_directory(output: &Path) -> Result<(), BuildError> {
        let Some(directory) = output.parent() else {
            return Ok(());
        };
        fs::create_dir_all(directory).map_err(|error| {
            Self::environment(&format!("create `{}`", directory.display()), &error)
        })
    }

    fn environment(action: &str, error: &io::Error) -> BuildError {
        BuildError::Environment(format!("cannot {action}: {error}"))
    }
}
