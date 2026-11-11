use super::entry_span::EntrySpan;
use super::exit_status::ExitStatus;
use super::interruptible_formatting::InterruptibleFormatting;
use super::source_report::SourceReport;
use super::stage_work::StageWork;
use crate::emit::{EmittedCrate, RustEmitter};
use crate::types::TypedProgram;

pub struct Emission<'a> {
    report: SourceReport<'a>,
    work: &'a mut StageWork,
    formatting: InterruptibleFormatting<'a>,
}

impl<'a> Emission<'a> {
    pub fn new(
        report: SourceReport<'a>,
        work: &'a mut StageWork,
        formatting: InterruptibleFormatting<'a>,
    ) -> Self {
        Self {
            report,
            work,
            formatting,
        }
    }

    pub fn emit(&mut self, program: &TypedProgram) -> Result<EmittedCrate, ExitStatus> {
        let emitted = RustEmitter::emit(program, EntrySpan::start())
            .map_err(|diagnostics| self.report.report(&diagnostics))?;
        self.work.emitted_expressions = Some(emitted.emission.emitted_expressions);
        let written = self.formatting.format_or_keep_unformatted(emitted)?;
        self.work.scanned_tokens = written
            .formatting
            .map(|formatting| formatting.scanned_tokens);
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::Emission;
    use crate::build::EmittedFormatting;
    use crate::cli::checking::Checking;
    use crate::cli::exit_status::ExitStatus;
    use crate::cli::expansion::Expansion;
    use crate::cli::fake_programs::FakePrograms;
    use crate::cli::interruptible_formatting::InterruptibleFormatting;
    use crate::cli::interruption::Interruption;
    use crate::cli::program_file::ProgramFile;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::source_report::SourceReport;
    use crate::cli::stage_work::StageWork;
    use crate::source::SourceMap;
    use crate::types::TypedProgram;

    struct EmissionFixture {
        sources: SourceMap,
        work: StageWork,
        output: RecordedOutput,
        interruption: Interruption,
    }

    impl EmissionFixture {
        const MODULE: &str = "geometry.knv";

        fn new() -> Self {
            Self {
                sources: SourceMap::default(),
                work: StageWork::default(),
                output: RecordedOutput::default(),
                interruption: Interruption::default(),
            }
        }

        fn checked(&mut self, text: &str) -> TypedProgram {
            let file = ProgramFile::beside_itself(PathBuf::from(Self::MODULE));
            let entry = self.sources.add(Self::MODULE, text);
            let program = Expansion::new(&mut self.sources, &mut self.work, &self.output)
                .expand(&file, entry)
                .expect("the fake program expands");
            Checking::new(
                SourceReport::new(&self.sources, &self.output),
                &mut self.work,
            )
            .check(&program)
            .expect("the fake program checks")
        }

        fn emission(&mut self) -> Emission<'_> {
            let formatting = EmittedFormatting::within(EmittedFormatting::TIME_LIMIT);
            Emission::new(
                SourceReport::new(&self.sources, &self.output),
                &mut self.work,
                InterruptibleFormatting::new(formatting, &self.interruption),
            )
        }
    }

    #[test]
    fn emits_formatted_rust_and_records_its_work() {
        let mut fixture = EmissionFixture::new();
        let program = fixture.checked(FakePrograms::PRINTS_A_SUM);
        let emitted = fixture.emission().emit(&program);
        assert!(emitted.is_ok_and(|emitted| emitted.formatting.is_some()));
        assert!(
            fixture
                .work
                .emitted_expressions
                .is_some_and(|count| count > 0)
        );
        assert!(fixture.work.scanned_tokens.is_some_and(|count| count > 0));
    }

    #[test]
    fn reports_a_program_that_does_not_emit() {
        let mut fixture = EmissionFixture::new();
        let program = fixture.checked(FakePrograms::WITHOUT_MAIN);
        let status = fixture.emission().emit(&program).err();
        assert_eq!(status, Some(ExitStatus::ProgramErrors));
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }
}
