use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::backend_verdicts::BackendVerdicts;
use crate::subject_program::SubjectProgram;

#[derive(Clone, Debug)]
pub struct Finding {
    pub oracle: &'static str,
    pub program: SubjectProgram,
    pub verdicts: BackendVerdicts,
    pub summary: String,
}

impl Finding {
    const PROGRAM_EXTENSION: &'static str = "knv";

    pub fn of(oracle: &'static str, verdicts: &BackendVerdicts, summary: String) -> Self {
        Self {
            oracle,
            program: verdicts.program.clone(),
            verdicts: verdicts.clone(),
            summary,
        }
    }

    pub fn write_program(&self, directory: &Path) -> io::Result<()> {
        let path = directory
            .join(&self.program.name)
            .with_extension(Self::PROGRAM_EXTENSION);
        fs::write(path, self.program.entry_text())
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.program.name, self.oracle, self.summary
        )
    }
}
