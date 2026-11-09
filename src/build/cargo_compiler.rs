use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

use super::BuildError;
use super::build_plan::BuildPlan;
use super::cargo_manifest::CargoManifest;
use super::interrupt_signal::InterruptSignal;
use super::rustc_errors::RustcDiagnosticMapper;
use crate::emit::EmittedCrate;
use crate::source::Span;

pub struct CargoCompiler {
    directory: PathBuf,
    plan: BuildPlan,
}

pub struct CargoTarget<'t> {
    pub manifest: CargoManifest<'t>,
    pub lock_file: PathBuf,
    pub output: &'t Path,
}

struct CargoReport {
    bin_name: String,
    program_messages: Vec<String>,
    dependency_errors: Vec<String>,
    executable: Option<PathBuf>,
}

impl CargoCompiler {
    const CARGO: &str = "cargo";
    const MANIFEST: &str = "Cargo.toml";
    const LOCK_FILE: &str = "Cargo.lock";
    const SOURCE: &str = "src/main.rs";
    const COMPILER_MESSAGE: &str = "compiler-message";
    const COMPILER_ARTIFACT: &str = "compiler-artifact";
    const ERROR_LEVEL: &str = "error";

    pub fn new(directory: PathBuf, plan: BuildPlan) -> Self {
        Self { directory, plan }
    }

    pub fn compile(
        &self,
        emitted: &EmittedCrate,
        target: &CargoTarget<'_>,
        entry: Span,
    ) -> Result<(), BuildError> {
        self.write_crate(emitted, target)
            .map_err(|error| Self::environment("write the generated crate", &error))?;
        let output = self.run_cargo(&target.manifest.bin_name)?;
        if InterruptSignal::stopped(output.status) {
            return Err(BuildError::Interrupted);
        }
        let report = CargoReport::read(
            &String::from_utf8_lossy(&output.stdout),
            &target.manifest.bin_name,
        );
        match (&report.executable, output.status.success()) {
            (Some(executable), true) => self.finish(executable, target),
            _ => Err(report.failure(emitted, entry, &output.stderr)),
        }
    }

    fn write_crate(&self, emitted: &EmittedCrate, target: &CargoTarget<'_>) -> io::Result<()> {
        let source = self.directory.join(Self::SOURCE);
        fs::create_dir_all(source.parent().unwrap_or(&self.directory))?;
        Self::write_if_changed(
            &self.directory.join(Self::MANIFEST),
            &target.manifest.render(&self.plan),
        )?;
        Self::write_if_changed(&source, &emitted.code)?;
        if target.lock_file.is_file() {
            fs::copy(&target.lock_file, self.directory.join(Self::LOCK_FILE))?;
        }
        Ok(())
    }

    fn write_if_changed(path: &Path, text: &str) -> io::Result<()> {
        if fs::read_to_string(path).is_ok_and(|written| written == text) {
            return Ok(());
        }
        fs::write(path, text)
    }

    fn run_cargo(&self, bin_name: &str) -> Result<Output, BuildError> {
        Command::new(Self::CARGO)
            .args(["rustc", "--bin", bin_name, "--message-format=json"])
            .args(self.plan.cargo_flags())
            .arg("--")
            .args(self.plan.lint_flags())
            .current_dir(&self.directory)
            .output()
            .map_err(|error| {
                Self::environment(
                    "run cargo, which builds a project with crate dependencies",
                    &error,
                )
            })
    }

    fn finish(&self, executable: &Path, target: &CargoTarget<'_>) -> Result<(), BuildError> {
        if let Some(directory) = target.output.parent() {
            fs::create_dir_all(directory)
                .map_err(|error| Self::environment("create the output directory", &error))?;
        }
        fs::copy(executable, target.output)
            .map_err(|error| Self::environment("copy the built program", &error))?;
        fs::copy(self.directory.join(Self::LOCK_FILE), &target.lock_file)
            .map_err(|error| Self::environment("copy `Cargo.lock` back", &error))?;
        Ok(())
    }

    fn environment(action: &str, error: &io::Error) -> BuildError {
        BuildError::Environment(format!("cannot {action}: {error}"))
    }
}

impl CargoReport {
    fn read(stdout: &str, bin_name: &str) -> Self {
        let mut report = Self {
            bin_name: bin_name.to_string(),
            program_messages: Vec::new(),
            dependency_errors: Vec::new(),
            executable: None,
        };
        stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .for_each(|record| report.add(&record));
        report
    }

    fn add(&mut self, record: &Value) {
        match record["reason"].as_str() {
            Some(CargoCompiler::COMPILER_MESSAGE) => self.add_message(record),
            Some(CargoCompiler::COMPILER_ARTIFACT) => self.add_artifact(record),
            _ => {}
        }
    }

    fn add_message(&mut self, record: &Value) {
        let message = &record["message"];
        if self.is_program(record) {
            return self.program_messages.push(message.to_string());
        }
        if message["level"].as_str() == Some(CargoCompiler::ERROR_LEVEL) {
            let rendered = message["rendered"].as_str().unwrap_or_default();
            let crate_name = record["target"]["name"].as_str().unwrap_or_default();
            self.dependency_errors
                .push(format!("crate `{crate_name}` failed to build:\n{rendered}"));
        }
    }

    fn add_artifact(&mut self, record: &Value) {
        if let Some(executable) = record["executable"].as_str() {
            self.executable = Some(PathBuf::from(executable));
        }
    }

    fn is_program(&self, record: &Value) -> bool {
        record["target"]["name"].as_str() == Some(self.bin_name.as_str())
    }

    fn failure(&self, emitted: &EmittedCrate, entry: Span, stderr: &[u8]) -> BuildError {
        let program_errors =
            RustcDiagnosticMapper::new(emitted, entry).errors_of(&self.program_messages);
        if program_errors.has_errors() {
            return BuildError::Program(program_errors);
        }
        if !self.dependency_errors.is_empty() {
            return BuildError::Environment(self.dependency_errors.join("\n"));
        }
        BuildError::Environment(String::from_utf8_lossy(stderr).trim_end().to_string())
    }
}
