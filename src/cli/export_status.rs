use super::exit_status::ExitStatus;

#[must_use = "a failed export check must end the command"]
pub struct ExportStatus {
    result: Result<(), ExitStatus>,
}

impl ExportStatus {
    pub fn new(result: Result<(), ExitStatus>) -> Self {
        Self { result }
    }

    pub fn result(self) -> Result<(), ExitStatus> {
        self.result
    }
}
