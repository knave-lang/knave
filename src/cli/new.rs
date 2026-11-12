use std::fs;
use std::path::{Path, PathBuf};

use super::doc_write::DocWriteCommand;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use super::working_directory::WorkingDirectory;
use crate::diagnostic::Diagnostics;
use crate::format::{ModuleLookup, SourceFormatter};
use crate::language::Language;
use crate::project::{ProjectName, ProjectReader};
use crate::source::Span;

pub struct NewCommand {
    name: String,
    formatter: SourceFormatter,
}

struct NewFile {
    path: PathBuf,
    text: String,
}

impl NewCommand {
    const ENTRY_TEXT: &str = "(defn main ()\n  (println \"hello\"))\n";
    const NAME_ARGUMENT: &str = "<name argument>";
    const IGNORE_FILE: &str = ".gitignore";

    pub fn new(name: String) -> Self {
        Self {
            name,
            formatter: SourceFormatter::default(),
        }
    }

    pub fn run(mut self, terminal: &mut Terminal) -> ExitStatus {
        let root = PathBuf::from(&self.name);
        match self.files(terminal, &root) {
            Ok(files) => Self::write_all(&root, &files),
            Err(status) => status,
        }
    }

    fn files(&mut self, terminal: &mut Terminal, root: &Path) -> Result<[NewFile; 3], ExitStatus> {
        self.check_name(terminal)?;
        Self::refuse_existing(root)?;
        let project_file = NewFile {
            path: root.join(Language::project_file_name()),
            text: format!("({} {})\n", ProjectReader::HEAD, self.name),
        };
        let mut sources = terminal.sources();
        let source = sources.add(project_file.path.display(), project_file.text.clone());
        let project = sources.read_project(source)?;
        let entry_file = NewFile {
            path: root.join(&project.entry),
            text: Self::ENTRY_TEXT.to_string(),
        };
        let working_directory = WorkingDirectory::path();
        Ok([
            self.formatted(
                terminal,
                project_file,
                &ModuleLookup::beside_project_file(working_directory),
            ),
            self.formatted(
                terminal,
                entry_file,
                &ModuleLookup::beside(working_directory),
            ),
            Self::ignore_file(root),
        ])
    }

    fn ignore_file(root: &Path) -> NewFile {
        NewFile {
            path: root.join(Self::IGNORE_FILE),
            text: format!("/{}/\n", DocWriteCommand::DEFAULT_FOLDER),
        }
    }

    fn check_name(&self, terminal: &mut Terminal) -> Result<(), ExitStatus> {
        let source = terminal
            .sources()
            .add(Self::NAME_ARGUMENT, self.name.clone());
        let whole_name = Span::new(source, 0..self.name.len());
        ProjectName::check(&self.name, whole_name).map_err(|problem| {
            let mut problems = Diagnostics::default();
            problems.push(problem);
            terminal.source_report().report(&problems)
        })
    }

    fn refuse_existing(root: &Path) -> Result<(), ExitStatus> {
        if !root.exists() {
            return Ok(());
        }
        StandardError::write_line(format_args!(
            "error: `{}` already exists\nhelp: choose another name, or remove it first",
            root.display()
        ));
        Err(ExitStatus::UsageError)
    }

    fn formatted(
        &mut self,
        terminal: &mut Terminal,
        file: NewFile,
        lookup: &ModuleLookup,
    ) -> NewFile {
        let source = terminal.sources().add(file.path.display(), file.text);
        let text = self
            .formatter
            .format(terminal.source_file(source), lookup)
            .unwrap_or_else(|failure| {
                unreachable!("the formatter reads and keeps a new project's files: {failure:?}")
            });
        NewFile {
            path: file.path,
            text,
        }
    }

    fn write_all(root: &Path, files: &[NewFile]) -> ExitStatus {
        match files.iter().find_map(|file| file.write().err()) {
            Some(failure) => {
                fs::remove_dir_all(root).ok();
                failure
            }
            None => ExitStatus::Success,
        }
    }
}

impl NewFile {
    fn write(&self) -> Result<(), ExitStatus> {
        fs::create_dir_all(ProgramFile::directory_of(&self.path))
            .and_then(|()| fs::write(&self.path, &self.text))
            .map_err(|error| ProgramFile::report_unwritable(&self.path, &error))
    }
}
