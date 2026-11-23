use super::exit_status::ExitStatus;
use super::project_program::ProjectProgram;
use crate::lint::ProjectLibrary;

pub struct LoadedPrograms {
    programs: Vec<ProjectProgram>,
    failure: Option<ExitStatus>,
    library: ProjectLibrary,
}

impl Default for LoadedPrograms {
    fn default() -> Self {
        Self {
            programs: Vec::new(),
            failure: None,
            library: ProjectLibrary::Unpublished,
        }
    }
}

impl LoadedPrograms {
    pub fn add(&mut self, loaded: Result<ProjectProgram, ExitStatus>) {
        match loaded {
            Ok(program) => self.programs.push(program),
            Err(status) => self.fail(status),
        }
    }

    pub fn fail(&mut self, status: ExitStatus) {
        self.failure = Some(status);
    }

    pub fn publish(&mut self, library: ProjectLibrary) {
        self.library = library;
    }

    pub fn library(&self) -> ProjectLibrary {
        self.library
    }

    pub fn into_parts(self) -> (Vec<ProjectProgram>, Option<ExitStatus>) {
        (self.programs, self.failure)
    }
}
