use std::path::PathBuf;

use super::checking::Checking;
use super::entry_span::EntrySpan;
use super::expansion::Expansion;
use super::program_file::ProgramFile;
use super::recorded_output::RecordedOutput;
use super::source_report::SourceReport;
use super::stage_work::StageWork;
use crate::emit::{EmittedCrate, RustEmitter};
use crate::expand::Program;
use crate::source::SourceMap;
use crate::types::TypedProgram;

pub struct FakeStages {
    sources: SourceMap,
    work: StageWork,
    output: RecordedOutput,
}

impl FakeStages {
    const MODULE: &str = "geometry.knv";

    pub fn expanded(text: &str) -> Program {
        Self::new().expand(text)
    }

    pub fn checked(text: &str) -> TypedProgram {
        let mut stages = Self::new();
        let program = stages.expand(text);
        Checking::new(
            SourceReport::new(&stages.sources, &stages.output),
            &mut stages.work,
        )
        .check(&program)
        .expect("the fake program checks")
    }

    pub fn emitted(text: &str) -> EmittedCrate {
        RustEmitter::emit(&Self::checked(text), EntrySpan::start()).expect("the fake program emits")
    }

    fn new() -> Self {
        Self {
            sources: SourceMap::default(),
            work: StageWork::default(),
            output: RecordedOutput::default(),
        }
    }

    fn expand(&mut self, text: &str) -> Program {
        let file = ProgramFile::beside_itself(PathBuf::from(Self::MODULE));
        let entry = self.sources.add(Self::MODULE, text);
        Expansion::new(&mut self.sources, &mut self.work, &self.output)
            .expand(&file, entry)
            .expect("the fake program expands")
    }
}
