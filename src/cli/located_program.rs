use std::path::PathBuf;

use super::exit_status::ExitStatus;
use super::library_walk::FetchingStore;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use super::working_directory::WorkingDirectory;
use crate::language::Language;

pub struct LocatedProgram {
    file: ProgramFile,
    project: Option<LocatedProject>,
}

impl LocatedProgram {
    pub fn of_file(file: PathBuf, terminal: &Terminal) -> Result<Self, ExitStatus> {
        let project = LocatedProject::search_from(
            &ProgramFile::directory_of(&file),
            terminal,
            FetchingStore::default(),
        )?;
        let file = match &project {
            Some(project) => project.placement().file_within(file),
            None => ProgramFile::beside_itself(file),
        };
        Ok(Self { file, project })
    }

    pub fn of_working_directory(terminal: &Terminal) -> Result<Self, ExitStatus> {
        let working_directory = WorkingDirectory::path();
        match LocatedProject::search_from(working_directory, terminal, FetchingStore::default())? {
            Some(project) => Ok(Self {
                file: project.entry(),
                project: Some(project),
            }),
            None => Err(Self::report_no_program()),
        }
    }

    pub fn file(&self) -> &ProgramFile {
        &self.file
    }

    pub fn into_file(self) -> ProgramFile {
        self.file
    }

    pub fn into_project(self) -> Option<LocatedProject> {
        self.project
    }

    pub fn project(&self) -> Option<&LocatedProject> {
        self.project.as_ref()
    }

    pub fn entry_project(&self) -> Option<&LocatedProject> {
        self.project
            .as_ref()
            .filter(|project| project.is_entry(&self.file))
    }

    fn report_no_program() -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: no file given, and no `{}` in this directory or a parent\n\
             help: name a file, or run the command inside a project",
            Language::project_file_name()
        ));
        ExitStatus::UsageError
    }
}
