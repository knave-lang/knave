use std::fs;
use std::io;
use std::path::Path;

use crate::backend_verdicts::BackendVerdicts;
use crate::citation::Citation;
use crate::finding_line::FindingLine;
use crate::finding_signature::FindingSignature;
use crate::subject_program::SubjectProgram;
use crate::verdict::Rejection;

#[derive(Clone, Debug)]
pub struct Finding {
    pub oracle: &'static str,
    pub program: SubjectProgram,
    pub verdicts: BackendVerdicts,
    pub summary: String,
    pub evidence: Vec<Rejection>,
}

impl Finding {
    const PROGRAM_EXTENSION: &'static str = "knv";

    pub fn of(oracle: &'static str, verdicts: &BackendVerdicts, citation: Citation) -> Self {
        Self {
            oracle,
            program: verdicts.program.clone(),
            verdicts: verdicts.clone(),
            summary: citation.summary(),
            evidence: citation.into_rejections(),
        }
    }

    pub fn signature(&self) -> FindingSignature {
        FindingSignature::new(self.oracle, &self.evidence)
    }

    pub fn line(&self) -> FindingLine {
        FindingLine::new(
            self.program.name.clone(),
            format!("{}: {}", self.oracle, self.summary),
        )
    }

    pub fn write_program(&self, directory: &Path) -> io::Result<()> {
        let path = directory
            .join(&self.program.name)
            .with_extension(Self::PROGRAM_EXTENSION);
        fs::write(path, self.program.entry_text())
    }
}
