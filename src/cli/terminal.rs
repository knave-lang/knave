mod source_loader;

use std::env::{self, VarError};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use anstream::ColorChoice;

use super::build_directory::BuildDirectory;
use super::checking::Checking;
use super::diagnostic_output::{DiagnosticOutput, RenderedDiagnostics};
use super::exit_status::ExitStatus;
use super::expansion::Expansion;
use super::interruption::Interruption;
use super::located_program::LocatedProgram;
use super::program_file::ProgramFile;
use super::project_task::ProjectTask;
use super::source_report::SourceReport;
use super::stage_work::StageWork;
use super::standard_error::StandardError;
use crate::build::{
    BuildError, BuildPlan, BuildProfile, CargoCompiler, CargoTarget, EmittedFormatting,
    FormattingError, RustCompiler,
};
use crate::emit::{EmittedCrate, RustEmitter};
use crate::language::Language;
use crate::source::Span;
use crate::source::{SourceFile, SourceId, SourceMap};
use crate::types::TypedProgram;
pub use source_loader::SourceLoader;

pub struct BuildOptions {
    pub profile: BuildProfile,
    pub output: Option<PathBuf>,
}

pub struct BuildTarget<'a> {
    pub directory: PathBuf,
    pub crate_name: &'a str,
    pub output: &'a Path,
}

pub enum BuildRoute<'a> {
    Rustc(BuildTarget<'a>),
    Cargo(CargoTarget<'a>),
}

pub struct Terminal {
    sources: SourceMap,
    work: StageWork,
    diagnostics: Rc<dyn DiagnosticOutput>,
    interruption: Interruption,
}

impl Terminal {
    pub const BUILD_PARENT: &str = "target";
    pub const WORKING_DIRECTORY: &str = ".";

    pub fn new(standard_error: ColorChoice) -> Self {
        Self::reporting_to(Rc::new(RenderedDiagnostics::new(standard_error)))
    }

    pub fn scratch(&self) -> Self {
        Self::reporting_to(self.diagnostic_output())
    }

    pub fn reporting_to(diagnostics: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            sources: SourceMap::default(),
            work: StageWork::default(),
            diagnostics,
            interruption: Interruption::default(),
        }
    }

    pub fn stop_before_rustc_on(&mut self, interruption: Interruption) {
        self.interruption = interruption;
    }

    pub fn report_diagnostics_to(&mut self, diagnostics: Rc<dyn DiagnosticOutput>) {
        self.diagnostics = diagnostics;
    }

    pub fn diagnostic_output(&self) -> Rc<dyn DiagnosticOutput> {
        Rc::clone(&self.diagnostics)
    }

    pub fn work(&self) -> &StageWork {
        &self.work
    }

    pub fn sources(&mut self) -> SourceLoader<'_> {
        SourceLoader::new(&mut self.sources, self.diagnostics.as_ref())
    }

    pub fn source_file(&self, source: SourceId) -> &SourceFile {
        self.sources.file(source)
    }

    pub fn expansion(&mut self) -> Expansion<'_> {
        Expansion::new(&mut self.sources, &mut self.work, self.diagnostics.as_ref())
    }

    pub fn checking(&mut self) -> Checking<'_> {
        Checking::new(
            SourceReport::new(&self.sources, self.diagnostics.as_ref()),
            &mut self.work,
        )
    }

    pub fn source_report(&self) -> SourceReport<'_> {
        SourceReport::new(&self.sources, self.diagnostics.as_ref())
    }

    pub fn build(
        &mut self,
        program: &LocatedProgram,
        options: &BuildOptions,
    ) -> Result<PathBuf, ExitStatus> {
        let emitted = self.emit(program.file())?;
        let crate_name = Self::crate_name(program.file().path());
        let (directory, output_name) = match program.entry_project() {
            Some(project) => (project.build_directory(), project.name().to_string()),
            None => (
                Self::build_directory(program.file().path(), &crate_name)?,
                crate_name.clone(),
            ),
        };
        let output = options
            .output
            .clone()
            .unwrap_or_else(|| directory.join(&output_name));
        let cargo = program.project().and_then(|project| {
            project
                .crates()
                .program_target(project.name(), directory.clone(), output.clone())
        });
        let route = match cargo {
            Some(target) => BuildRoute::Cargo(target),
            None => BuildRoute::Rustc(BuildTarget {
                directory,
                crate_name: &crate_name,
                output: &output,
            }),
        };
        self.compile_into(&emitted, &route, options.profile)?;
        Ok(output)
    }

    pub fn build_task(
        &mut self,
        emitted: &EmittedCrate,
        task: &ProjectTask<'_>,
        profile: BuildProfile,
    ) -> Result<PathBuf, ExitStatus> {
        self.compile_into(emitted, &task.route(), profile)?;
        Ok(task.binary().to_path_buf())
    }

    pub fn compile_into(
        &self,
        emitted: &EmittedCrate,
        route: &BuildRoute<'_>,
        profile: BuildProfile,
    ) -> Result<(), ExitStatus> {
        match route {
            BuildRoute::Rustc(target) => self.compile(emitted, target, profile),
            BuildRoute::Cargo(target) => self.compile_with_cargo(emitted, target, profile),
        }
    }

    fn compile(
        &self,
        emitted: &EmittedCrate,
        target: &BuildTarget<'_>,
        profile: BuildProfile,
    ) -> Result<(), ExitStatus> {
        if self.interruption.is_raised() {
            return Err(self.report_build(BuildError::Interrupted));
        }
        RustCompiler::new(target.directory.clone(), profile)
            .compile(
                emitted,
                target.crate_name,
                target.output,
                Self::entry_span(),
            )
            .map_err(|error| self.report_build(self.interrupted_or(error)))
    }

    fn compile_with_cargo(
        &self,
        emitted: &EmittedCrate,
        target: &CargoTarget<'_>,
        profile: BuildProfile,
    ) -> Result<(), ExitStatus> {
        if self.interruption.is_raised() {
            return Err(self.report_build(BuildError::Interrupted));
        }
        let waiting = |target: &str| {
            self.source_report()
                .note(&format!("waiting for another build in {target}"));
        };
        CargoCompiler::new(target.directory.clone(), BuildPlan::new(profile), &waiting)
            .compile(emitted, target, Self::entry_span())
            .map_err(|error| self.report_build(self.interrupted_or(error)))
    }

    fn interrupted_or(&self, error: BuildError) -> BuildError {
        if self.interruption.caused_a_failure() {
            return BuildError::Interrupted;
        }
        error
    }

    pub fn emit(&mut self, file: &ProgramFile) -> Result<EmittedCrate, ExitStatus> {
        let formatting = Self::formatting()?;
        let program = self.check_file(file)?;
        self.emit_formatted(&program, &formatting)
    }

    pub fn emit_checked(&mut self, program: &TypedProgram) -> Result<EmittedCrate, ExitStatus> {
        let formatting = Self::formatting()?;
        self.emit_formatted(program, &formatting)
    }

    fn emit_formatted(
        &mut self,
        program: &TypedProgram,
        formatting: &EmittedFormatting,
    ) -> Result<EmittedCrate, ExitStatus> {
        let emitted = RustEmitter::emit(program, Self::entry_span())
            .map_err(|diagnostics| self.source_report().report(&diagnostics))?;
        self.work.emitted_expressions = Some(emitted.emission.emitted_expressions);
        let written = match formatting.apply(&emitted) {
            Ok(formatted) => formatted,
            Err(FormattingError::Interrupted) => {
                return Err(self.report_build(BuildError::Interrupted));
            }
            Err(_) if self.interruption.caused_a_failure() => {
                return Err(self.report_build(BuildError::Interrupted));
            }
            Err(error) => {
                StandardError::write_line(format_args!("note: {error}; writing unformatted Rust"));
                emitted
            }
        };
        self.work.scanned_tokens = written
            .formatting
            .map(|formatting| formatting.scanned_tokens);
        Ok(written)
    }

    fn formatting() -> Result<EmittedFormatting, ExitStatus> {
        Self::rustfmt_time_limit().map(EmittedFormatting::within)
    }

    fn rustfmt_time_limit() -> Result<Duration, ExitStatus> {
        let variable = EmittedFormatting::time_limit_variable();
        match env::var(&variable) {
            Err(VarError::NotPresent) => Ok(EmittedFormatting::TIME_LIMIT),
            value => value
                .ok()
                .and_then(|milliseconds| milliseconds.parse().ok())
                .map(Duration::from_millis)
                .ok_or_else(|| Self::report_malformed_variable(&variable)),
        }
    }

    pub fn check_file(&mut self, file: &ProgramFile) -> Result<TypedProgram, ExitStatus> {
        let entry = self.sources().load(file.path())?;
        let program = self.expansion().expand(file, entry)?;
        self.checking().check(&program)
    }

    fn report_build(&self, error: BuildError) -> ExitStatus {
        match error {
            BuildError::Program(diagnostics) => self.source_report().report(&diagnostics),
            BuildError::Environment(message) => {
                StandardError::write_line(format_args!("error: {message}"));
                ExitStatus::Environment
            }
            BuildError::Interrupted => ExitStatus::Interrupted,
        }
    }

    fn build_directory(path: &Path, crate_name: &str) -> Result<PathBuf, ExitStatus> {
        let working_directory = Self::canonical(Path::new(Self::WORKING_DIRECTORY))?;
        let entry_directory = Self::canonical(&ProgramFile::directory_of(path))?;
        Ok(BuildDirectory::new(Self::build_root(), working_directory)
            .for_entry(&entry_directory, crate_name))
    }

    pub fn build_root() -> PathBuf {
        Path::new(Self::BUILD_PARENT).join(Language::NAME)
    }

    pub fn canonical(directory: &Path) -> Result<PathBuf, ExitStatus> {
        Path::new(Self::WORKING_DIRECTORY)
            .join(directory)
            .canonicalize()
            .map_err(|error| Self::report_unresolvable(directory, &error))
    }

    fn entry_span() -> Span {
        Span::new(SourceId::new(0), 0..0)
    }

    pub fn crate_name(path: &Path) -> String {
        Self::module_name(path).replace(|character: char| !character.is_ascii_alphanumeric(), "_")
    }

    fn module_name(path: &Path) -> String {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn report_unresolvable(directory: &Path, error: &std::io::Error) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: cannot resolve `{}`: {error}",
            directory.display()
        ));
        ExitStatus::UsageError
    }

    fn report_malformed_variable(variable: &str) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: `{variable}` must be a whole number of milliseconds"
        ));
        ExitStatus::UsageError
    }
}
