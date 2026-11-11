use std::fs;
use std::path::Path;

use super::crate_section::CrateSection;
use super::exit_status::ExitStatus;
use super::library_walk::FetchingStore;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::format::{ModuleLookup, SourceFormatter};
use crate::language::Language;
use crate::project::{CrateDependency, ProjectEditor};

pub struct ProjectFileEdit {
    project: LocatedProject,
}

impl ProjectFileEdit {
    const STAGED_SUFFIX: &str = ".tmp";

    pub fn of_working_directory(terminal: &Terminal, command: &str) -> Result<Self, ExitStatus> {
        let working_directory = Path::new(Terminal::WORKING_DIRECTORY);
        match LocatedProject::search_from(working_directory, terminal, FetchingStore::default())? {
            Some(project) => Ok(Self { project }),
            None => Err(Self::report_no_project(command)),
        }
    }

    pub fn listed_crate(&self, section: CrateSection, name: &str) -> Option<&CrateDependency> {
        self.project.own_crate(section, name)
    }

    pub fn apply(
        &self,
        terminal: &Terminal,
        edit: impl Fn(&ProjectEditor<'_>) -> Option<String>,
    ) -> Result<bool, ExitStatus> {
        let path = self.project.project_file();
        let mut scratch = terminal.scratch();
        let source = scratch.sources().load(&path)?;
        let text = scratch.source_file(source).text().to_string();
        let forms = scratch.sources().read(source)?;
        let editor =
            ProjectEditor::of(&text, &forms).expect("a loaded project file holds a project form");
        let Some(edited) = edit(&editor) else {
            return Ok(false);
        };
        let formatted = self.formatted(&mut scratch, &path, edited);
        Self::replace(&path, &formatted)?;
        Ok(true)
    }

    fn replace(path: &Path, text: &str) -> Result<(), ExitStatus> {
        Self::replace_target(path, text)
            .map_err(|error| ProgramFile::report_unwritable(path, &error))
    }

    fn replace_target(path: &Path, text: &str) -> std::io::Result<()> {
        let path = &fs::canonicalize(path)?;
        let mut staged = path.as_os_str().to_os_string();
        staged.push(Self::STAGED_SUFFIX);
        let staged = Path::new(&staged);
        fs::write(staged, text)?;
        fs::set_permissions(staged, fs::metadata(path)?.permissions())?;
        fs::rename(staged, path)
    }

    fn formatted(&self, scratch: &mut Terminal, path: &Path, text: String) -> String {
        let directory = ProgramFile::directory_of(path);
        let root = ProgramFile::canonical(&directory).unwrap_or(directory);
        let source = scratch.sources().add(path, text);
        SourceFormatter::default()
            .format(
                scratch.source_file(source),
                &ModuleLookup::beside_project_file(&root),
            )
            .unwrap_or_else(|failure| {
                unreachable!("the formatter reads and keeps an edited project file: {failure:?}")
            })
    }

    fn report_no_project(command: &str) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: no `{}` in this directory or a parent\nhelp: run `{} {command}` inside a project",
            Language::project_file_name(),
            Language::NAME
        ));
        ExitStatus::UsageError
    }
}
