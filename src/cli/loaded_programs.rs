use super::exit_status::ExitStatus;
use super::project_program::ProjectProgram;

#[derive(Default)]
pub struct LoadedPrograms {
    results: Vec<Result<ProjectProgram, ExitStatus>>,
}

impl LoadedPrograms {
    pub fn add(&mut self, loaded: Result<ProjectProgram, ExitStatus>) {
        self.results.push(loaded);
    }

    pub fn fail(&mut self, status: ExitStatus) {
        self.add(Err(status));
    }

    pub fn into_results(self) -> Vec<Result<ProjectProgram, ExitStatus>> {
        self.results
    }
}
