use std::fs;
use std::io;
use std::path::Path;

use crate::backend_verdicts::BackendVerdicts;
use crate::citation::Citation;
use crate::finding_line::FindingLine;
use crate::finding_signature::FindingSignature;
use crate::known_classes::KnownClasses;
use crate::subject_program::SubjectProgram;

#[derive(Clone, Debug)]
pub struct Finding {
    pub oracle: &'static str,
    pub program: SubjectProgram,
    pub verdicts: BackendVerdicts,
    citation: Citation,
}

impl Finding {
    const PROGRAM_EXTENSION: &'static str = "knv";

    pub fn of(oracle: &'static str, verdicts: &BackendVerdicts, citation: Citation) -> Self {
        Self {
            oracle,
            program: verdicts.program.clone(),
            verdicts: verdicts.clone(),
            citation,
        }
    }

    pub fn summary(&self) -> String {
        self.citation.summary()
    }

    pub fn signature(&self) -> FindingSignature {
        self.citation.signature(self.oracle)
    }

    pub fn is_explained_by(&self, classes: &KnownClasses) -> bool {
        self.citation.is_explained_by(classes)
    }

    pub fn line(&self) -> FindingLine {
        FindingLine::new(
            self.program.name.clone(),
            format!("{}: {}", self.oracle, self.summary()),
        )
    }

    pub fn write_program(&self, directory: &Path) -> io::Result<()> {
        let path = directory
            .join(&self.program.name)
            .with_extension(Self::PROGRAM_EXTENSION);
        fs::write(path, self.program.entry_text())
    }
}
