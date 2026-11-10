use std::fs;
use std::io;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;

use serde_json::Value;

use super::BuildError;
use super::build_plan::BuildPlan;
use super::cargo_manifest::CargoManifest;
use super::interrupt_signal::InterruptSignal;
use super::rustc_errors::RustcDiagnosticMapper;
use super::user_cache::UserCache;
use crate::emit::EmittedCrate;
use crate::source::Span;

pub struct CargoCompiler<'w> {
    directory: PathBuf,
    plan: BuildPlan,
    waiting: &'w dyn Fn(&str),
}

pub struct CargoTarget<'t> {
    pub manifest: CargoManifest<'t>,
    pub lock_file: PathBuf,
    pub output: &'t Path,
}

struct CargoReport {
    target_name: String,
    program_messages: Vec<String>,
    dependency_errors: Vec<String>,
    executable: Option<PathBuf>,
}

impl<'w> CargoCompiler<'w> {
    const CARGO: &'static str = "cargo";
    const TARGET_VARIABLE: &'static str = "CARGO_TARGET_DIR";
    const USER_TARGET_VARIABLES: [&'static str; 2] = ["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"];
    const TARGET_DIRECTORY: &'static str = "target";
    const TARGET_LOCKS: [&'static str; 2] = [
        "Blocking waiting for file lock on build directory",
        "Blocking waiting for file lock on artifact directory",
    ];
    const OWN_TARGET: &'static str = "Cargo's target directory";
    const CARGO_USE: &'static str = "run cargo, which builds a project with crate dependencies";
    const MANIFEST: &'static str = "Cargo.toml";
    const LOCK_FILE: &'static str = "Cargo.lock";
    const SOURCE: &'static str = "src/main.rs";
    const COMPILER_MESSAGE: &'static str = "compiler-message";
    const COMPILER_ARTIFACT: &'static str = "compiler-artifact";
    const ERROR_LEVEL: &'static str = "error";

    pub fn new(directory: PathBuf, plan: BuildPlan, waiting: &'w dyn Fn(&str)) -> Self {
        Self {
            directory,
            plan,
            waiting,
        }
    }

    pub fn compile(
        &self,
        emitted: &EmittedCrate,
        target: &CargoTarget<'_>,
        entry: Span,
    ) -> Result<(), BuildError> {
        self.write_crate(emitted, target)
            .map_err(|error| Self::environment("write the generated crate", &error))?;
        let output = self.run_cargo(&target.manifest)?;
        if InterruptSignal::stopped(output.status) {
            return Err(BuildError::Interrupted);
        }
        let report = CargoReport::read(
            &String::from_utf8_lossy(&output.stdout),
            &target.manifest.target_name,
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

    fn run_cargo(&self, manifest: &CargoManifest<'_>) -> Result<Output, BuildError> {
        let mut command = Command::new(Self::CARGO);
        command
            .arg("rustc")
            .args([manifest.target.cargo_flag(), &manifest.target_name])
            .arg("--message-format=json")
            .args(self.plan.cargo_flags())
            .arg("--")
            .args(self.plan.lint_flags())
            .current_dir(&self.directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let shared_target = Self::shared_target();
        if let Some(target) = &shared_target {
            command.env(Self::TARGET_VARIABLE, target);
        }
        self.watched(&mut command, shared_target)
            .map_err(|error| Self::environment(Self::CARGO_USE, &error))
    }

    fn watched(&self, command: &mut Command, target: Option<PathBuf>) -> io::Result<Output> {
        let mut child = command.spawn()?;
        let mut stdout = child
            .stdout
            .take()
            .expect("cargo's standard output is piped");
        let reader = thread::spawn(move || {
            let mut text = Vec::new();
            stdout.read_to_end(&mut text).map(|_| text)
        });
        let stderr = child
            .stderr
            .take()
            .expect("cargo's standard error is piped");
        let mut stderr_text = Vec::new();
        for line in BufReader::new(stderr).lines() {
            let line = line?;
            if Self::TARGET_LOCKS.iter().any(|lock| line.contains(lock)) {
                (self.waiting)(&Self::target_name(target.as_deref()));
            }
            stderr_text.extend_from_slice(line.as_bytes());
            stderr_text.push(b'\n');
        }
        let status = child.wait()?;
        let stdout_text = reader
            .join()
            .expect("the reader of cargo's output does not panic")?;
        Ok(Output {
            status,
            stdout: stdout_text,
            stderr: stderr_text,
        })
    }

    fn target_name(target: Option<&Path>) -> String {
        target.map_or_else(
            || Self::OWN_TARGET.to_string(),
            |target| format!("`{}`", target.display()),
        )
    }

    fn shared_target() -> Option<PathBuf> {
        let user_target = Self::USER_TARGET_VARIABLES
            .iter()
            .any(|name| UserCache::variable_path(name).is_some());
        Self::target_for(user_target, UserCache::directory())
    }

    fn target_for(user_target_set: bool, cache: Option<PathBuf>) -> Option<PathBuf> {
        if user_target_set {
            return None;
        }
        cache.map(|cache| cache.join(Self::TARGET_DIRECTORY))
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
    fn read(stdout: &str, target_name: &str) -> Self {
        let mut report = Self {
            target_name: target_name.to_string(),
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
        record["target"]["name"].as_str() == Some(self.target_name.as_str())
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CargoCompiler;

    #[test]
    fn the_shared_target_sits_in_the_user_cache_unless_cargo_target_dir_is_set() {
        let cache = Some(PathBuf::from("/cache/knave"));
        assert_eq!(
            CargoCompiler::target_for(false, cache.clone()),
            Some(PathBuf::from("/cache/knave/target"))
        );
        assert_eq!(CargoCompiler::target_for(true, cache), None);
    }

    #[test]
    fn without_a_cache_directory_cargo_keeps_its_own_target() {
        assert_eq!(CargoCompiler::target_for(false, None), None);
    }
}
