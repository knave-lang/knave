use std::fs;
use std::io;
use std::path::Path;

use crate::finding::Finding;
use crate::finding_signature::FindingSignature;
use crate::reported_signatures::ReportedSignatures;
use crate::subject_program::SubjectProgram;

pub struct ReducedFinding {
    finding: Finding,
    program: SubjectProgram,
}

impl ReducedFinding {
    const SUFFIX: &'static str = "reduced.knv";

    pub fn new(finding: Finding, program: SubjectProgram) -> Self {
        Self { finding, program }
    }

    pub fn record(&self, directory: &Path) -> io::Result<()> {
        let path = directory.join(format!("{}.{}", self.finding.program.name, Self::SUFFIX));
        fs::write(path, self.program.entry_text())
    }

    pub fn signature(&self) -> FindingSignature {
        self.finding.signature()
    }

    pub fn section(&self) -> String {
        format!(
            "### `{}`\n\n{}\n{}\n\n```lisp\n{}```\n",
            self.finding.program.name,
            ReportedSignatures::marker(&self.signature()),
            self.finding.line(),
            self.program.entry_text()
        )
    }
}
