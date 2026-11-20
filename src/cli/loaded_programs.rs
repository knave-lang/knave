use super::exit_status::ExitStatus;
use super::project_program::ProjectProgram;

#[derive(Default)]
pub struct LoadedPrograms {
    programs: Vec<ProjectProgram>,
    failure: Option<ExitStatus>,
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

    pub fn into_parts(self) -> (Vec<ProjectProgram>, Option<ExitStatus>) {
        (self.programs, self.failure)
    }
}
