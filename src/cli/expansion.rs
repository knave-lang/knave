use super::diagnostic_output::DiagnosticOutput;
use super::error_location::ModuleLoader;
use super::evaluation_request::EvaluationRequest;
use super::exit_status::ExitStatus;
use super::module_of_sources::ModuleOfSources;
use super::program_file::ProgramFile;
use super::sources_survey::SourcesSurvey;
use super::stage_work::StageWork;
use crate::diagnostic::Diagnostics;
use crate::expand::{ClassifiedForm, Expander, ExpansionSession, PreludeState, Program};
use crate::source::{SourceId, SourceMap, Span};

pub struct Expansion<'a> {
    sources: &'a mut SourceMap,
    work: &'a mut StageWork,
    output: &'a dyn DiagnosticOutput,
}

impl<'a> Expansion<'a> {
    const SAVED_PRELUDE: &'static [u8] = include_bytes!(env!("PRELUDE_STATE"));

    pub fn new(
        sources: &'a mut SourceMap,
        work: &'a mut StageWork,
        output: &'a dyn DiagnosticOutput,
    ) -> Self {
        Self {
            sources,
            work,
            output,
        }
    }

    pub fn expand(&mut self, file: &ProgramFile, entry: SourceId) -> Result<Program, ExitStatus> {
        let program = self
            .expander(file)
            .expand(file.module_path(), entry)
            .map_err(|diagnostics| self.report(&diagnostics))?;
        self.work.lowered_forms = Some(program.expansion.lowered_forms);
        Ok(program)
    }

    pub fn within_session<T>(
        &mut self,
        module: &ModuleOfSources,
        work: impl FnOnce(ExpansionSession<'_>) -> Result<T, Diagnostics>,
    ) -> Result<T, ExitStatus> {
        self.expander(&module.file)
            .open(module.file.module_path(), &module.sources)
            .and_then(work)
            .map_err(|diagnostics| self.report(&diagnostics))
    }

    pub fn within_evaluation<T>(
        &mut self,
        request: &EvaluationRequest,
        work: impl FnOnce(ExpansionSession<'_>) -> Result<T, Diagnostics>,
    ) -> Result<T, ExitStatus> {
        let module = &request.module;
        self.expander(&module.file)
            .evaluating(request.evaluated.iter().copied())
            .open(module.file.module_path(), &module.sources)
            .and_then(work)
            .map_err(|diagnostics| self.report(&diagnostics))
    }

    pub fn classify(&mut self, module: &ModuleOfSources, form: Span) -> ClassifiedForm {
        self.expander(&module.file)
            .classify(module.file.module_path(), &module.sources, form)
    }

    pub fn survey(&mut self, module: &ModuleOfSources, inputs: &[Span]) -> SourcesSurvey {
        let survey =
            self.expander(&module.file)
                .survey(module.file.module_path(), &module.sources, inputs);
        let loaders = survey
            .loaded
            .iter()
            .map(|loaded| ModuleLoader {
                file: self.sources.file(loaded.source).name().to_string(),
                input: loaded.input.source(),
            })
            .collect();
        SourcesSurvey::new(survey.program, survey.names, loaders)
    }

    fn expander(&mut self, file: &ProgramFile) -> Expander<'_> {
        let prelude =
            PreludeState::load(Self::SAVED_PRELUDE).expect("the embedded prelude state loads");
        Expander::with_prelude(self.sources, file.module_sources(), prelude)
    }

    fn report(&self, diagnostics: &Diagnostics) -> ExitStatus {
        self.output.report(self.sources, diagnostics)
    }
}
