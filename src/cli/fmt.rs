mod project_load_warning;

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::exit_status::ExitStatus;
use super::library_walk::LibraryStore;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::skipped_directory::SkippedDirectory;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::format::{FormatFailure, ModuleLookup, SourceFormatter};
use crate::language::Language;
use crate::modules::ModuleSources;
use project_load_warning::ProjectLoadWarning;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FmtMode {
    Write,
    Check,
}

pub struct FmtCommand {
    paths: Vec<PathBuf>,
    mode: FmtMode,
    standard_input_path: Option<PathBuf>,
    formatter: SourceFormatter,
    lookups: NearbyProjects,
}

struct WorstStatus {
    status: ExitStatus,
}

struct SourceFiles {
    skipped: SkippedDirectory,
}

struct NearbyProjects {
    projects: FxHashMap<PathBuf, Option<LocatedProject>>,
}

impl FmtCommand {
    pub const STANDARD_INPUT: &'static str = "-";
    const STANDARD_INPUT_NAME: &'static str = "<stdin>";

    pub fn new(paths: Vec<PathBuf>, mode: FmtMode, standard_input_path: Option<PathBuf>) -> Self {
        Self {
            paths,
            mode,
            standard_input_path,
            formatter: SourceFormatter::default(),
            lookups: NearbyProjects::new(),
        }
    }

    pub fn run(mut self, terminal: &mut Terminal) -> ExitStatus {
        match self.reads_standard_input() {
            Ok(true) => self.format_standard_input(terminal),
            Ok(false) => self.format_paths(terminal),
            Err(status) => status,
        }
    }

    fn reads_standard_input(&self) -> Result<bool, ExitStatus> {
        let dashes = self
            .paths
            .iter()
            .filter(|path| path.as_os_str() == Self::STANDARD_INPUT)
            .count();
        match (dashes, self.paths.len()) {
            (0, _) if self.standard_input_path.is_none() => Ok(false),
            (1, 1) => Ok(true),
            _ => Err(Self::report_misused_standard_input()),
        }
    }

    fn format_standard_input(&mut self, terminal: &mut Terminal) -> ExitStatus {
        let text = match ModuleSources::read_standard_input() {
            Ok(text) => text,
            Err(error) => return ExitStatus::report_unreadable_input(&error),
        };
        let name = self
            .standard_input_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(Self::STANDARD_INPUT_NAME));
        let lookup = self.lookups.lookup_for(&name, terminal);
        match self.formatted(terminal, &name, text.clone(), &lookup) {
            Ok(formatted) => self.answer_standard_input(terminal, &name, &text, &formatted),
            Err(status) => status,
        }
    }

    fn answer_standard_input(
        &self,
        terminal: &Terminal,
        name: &Path,
        text: &str,
        formatted: &str,
    ) -> ExitStatus {
        match self.mode {
            FmtMode::Write => terminal.write_output(formatted),
            FmtMode::Check if formatted == text => ExitStatus::Success,
            FmtMode::Check => self.apply(terminal, name, formatted),
        }
    }

    fn format_paths(&mut self, terminal: &mut Terminal) -> ExitStatus {
        match self.walked_paths() {
            Ok(paths) => self.format_walked(terminal, paths),
            Err(status) => status,
        }
    }

    fn walked_paths(&mut self) -> Result<Vec<PathBuf>, ExitStatus> {
        let paths = std::mem::take(&mut self.paths);
        if !paths.is_empty() {
            return Ok(paths);
        }
        let working_directory = Terminal::canonical(Path::new(Terminal::WORKING_DIRECTORY))?;
        let root = LocatedProject::canonical_root(&working_directory)
            .ok_or_else(LocatedProject::report_no_path)?;
        Ok(vec![Self::upward_path(&working_directory, &root)])
    }

    fn upward_path(from: &Path, ancestor: &Path) -> PathBuf {
        let depth = from
            .strip_prefix(ancestor)
            .map_or(0, |rest| rest.components().count());
        std::iter::repeat_n(Component::ParentDir, depth).collect()
    }

    fn format_walked(&mut self, terminal: &mut Terminal, paths: Vec<PathBuf>) -> ExitStatus {
        let mut worst = WorstStatus::new();
        for path in paths {
            match SourceFiles::under(&path) {
                Ok(files) => files
                    .iter()
                    .for_each(|file| worst.record(self.format_file(terminal, file))),
                Err(error) => worst.record(Terminal::report_unreadable(&path, &error)),
            }
        }
        worst.status
    }

    fn format_file(&mut self, terminal: &mut Terminal, path: &Path) -> ExitStatus {
        let text = match ModuleSources::read_file(path) {
            Ok(text) => text,
            Err(error) => return Terminal::report_unreadable(path, &error),
        };
        let lookup = self.lookups.lookup_for(path, terminal);
        match self.formatted(terminal, path, text.clone(), &lookup) {
            Ok(formatted) if formatted == text => ExitStatus::Success,
            Ok(formatted) => self.apply(terminal, path, &formatted),
            Err(status) => status,
        }
    }

    fn formatted(
        &mut self,
        terminal: &mut Terminal,
        path: &Path,
        text: String,
        lookup: &ModuleLookup,
    ) -> Result<String, ExitStatus> {
        let source = terminal.add_source(path, text);
        self.formatter
            .format(terminal.source_file(source), lookup)
            .map_err(|failure| Self::report_failure(terminal, path, failure))
    }

    fn apply(&self, terminal: &Terminal, path: &Path, formatted: &str) -> ExitStatus {
        match self.mode {
            FmtMode::Check => match terminal.write_output(format!("{}\n", path.display())) {
                ExitStatus::Success => ExitStatus::ProgramErrors,
                failure => failure,
            },
            FmtMode::Write => match fs::write(path, formatted) {
                Ok(()) => ExitStatus::Success,
                Err(error) => ProgramFile::report_unwritable(path, &error),
            },
        }
    }

    fn report_failure(terminal: &Terminal, path: &Path, failure: FormatFailure) -> ExitStatus {
        let fault = match failure {
            FormatFailure::Unreadable(diagnostics) => return terminal.report(&diagnostics),
            FormatFailure::ChangedMeaning => "its formatted text reads to other forms",
            FormatFailure::Unstable => "formatting its formatted text changes it again",
        };
        StandardError::write_line(format_args!(
            "error: internal formatter error in `{}`: {fault}, so the file is left as it was",
            path.display()
        ));
        ExitStatus::ProgramErrors
    }

    fn report_misused_standard_input() -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: `{}` reads standard input and takes no other path, and `--stdin-path` needs it",
            Self::STANDARD_INPUT
        ));
        ExitStatus::UsageError
    }
}

impl WorstStatus {
    fn new() -> Self {
        Self {
            status: ExitStatus::Success,
        }
    }

    fn record(&mut self, status: ExitStatus) {
        if Self::severity(status) > Self::severity(self.status) {
            self.status = status;
        }
    }

    fn severity(status: ExitStatus) -> u8 {
        match status {
            ExitStatus::Success => 0,
            ExitStatus::ProgramErrors => 1,
            _ => 2,
        }
    }
}

impl SourceFiles {
    const CURRENT_DIRECTORY: &'static str = ".";

    fn under(path: &Path) -> io::Result<Vec<PathBuf>> {
        let listed = Self::listed(path);
        if !listed.is_dir() {
            return Ok(vec![path.to_path_buf()]);
        }
        let skipped = SkippedDirectory::for_walk_of(listed);
        Self { skipped }.walk(path)
    }

    fn listed(path: &Path) -> &Path {
        if path.as_os_str().is_empty() {
            return Path::new(Self::CURRENT_DIRECTORY);
        }
        path
    }

    fn walk(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let mut entries: Vec<PathBuf> = fs::read_dir(Self::listed(path))?
            .map(|entry| entry.map(|entry| path.join(entry.file_name())))
            .collect::<io::Result<_>>()?;
        entries.sort();
        let mut files = Vec::new();
        for entry in entries {
            if Self::is_real_directory(&entry) {
                files.extend(self.walk_subdirectory(&entry)?);
            } else if Self::is_source(&entry) {
                files.push(entry);
            }
        }
        Ok(files)
    }

    fn walk_subdirectory(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        if self.skipped.is_skipped(directory) {
            return Ok(Vec::new());
        }
        self.walk(directory)
    }

    fn is_real_directory(path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
    }

    fn is_source(path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == Language::EXTENSION)
    }
}

impl NearbyProjects {
    fn new() -> Self {
        Self {
            projects: FxHashMap::default(),
        }
    }

    fn lookup_for(&mut self, path: &Path, terminal: &Terminal) -> ModuleLookup {
        let directory = Self::absolute_directory_of(path);
        if ProgramFile::is_project_file(path) {
            return ModuleLookup::beside_project_file(&directory);
        }
        match self.project_around(&directory, terminal) {
            Some(project) => {
                let file = project.placement().placed_file(path.to_path_buf());
                ModuleLookup::within_project(file.module_sources(), &directory)
            }
            None => ModuleLookup::beside(&directory),
        }
    }

    fn absolute_directory_of(path: &Path) -> PathBuf {
        let directory = ProgramFile::directory_of(path);
        let working_directory = Path::new(Terminal::WORKING_DIRECTORY);
        fs::canonicalize(working_directory.join(&directory)).unwrap_or(directory)
    }

    fn project_around(&mut self, directory: &Path, terminal: &Terminal) -> Option<&LocatedProject> {
        let root = LocatedProject::canonical_root(directory)?;
        self.projects
            .entry(root)
            .or_insert_with_key(|root| Self::load(root, terminal))
            .as_ref()
    }

    fn load(root: &Path, terminal: &Terminal) -> Option<LocatedProject> {
        let warning = Rc::new(ProjectLoadWarning::new(root, terminal.diagnostic_output()));
        let loading = Terminal::reporting_to(warning.clone());
        let loaded = LocatedProject::search_from(root, &loading, LibraryStore::cache_only());
        if loaded.is_err() {
            warning.warn_once();
        }
        loaded.ok().flatten()
    }
}
