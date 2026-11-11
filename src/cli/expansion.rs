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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::Expansion;
    use crate::cli::evaluation_request::EvaluationRequest;
    use crate::cli::exit_status::ExitStatus;
    use crate::cli::module_of_sources::ModuleOfSources;
    use crate::cli::program_file::ProgramFile;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::stage_work::StageWork;
    use crate::cli::temporary_directory::TemporaryDirectory;
    use crate::expand::ClassifiedForm;
    use crate::source::{SourceId, SourceMap, Span};

    struct ExpansionFixture {
        sources: SourceMap,
        work: StageWork,
        output: RecordedOutput,
        file: ProgramFile,
    }

    impl ExpansionFixture {
        const MODULE: &str = "geometry.knv";
        const DEFINITIONS: &str = "(def width 1)\n(def height 2)\n";
        const FIRST_DEFINITION: &str = "(def width 1)";
        const SECOND_DEFINITION: &str = "(def height 2)";
        const UNBOUND: &str = "(def width missing-name)\n";
        const MISTYPED: &str = "(def width (+ 1 \"two\"))\n";
        const UNREACHABLE_ARM: &str =
            "(enum shape (circle f64) point)\n\n(defn area (s)\n  (match s (_ 0.0) (point 1.0)))\n";
        const LOADED_MODULE: &str = "shapes";
        const LOADED_TEXT: &str = "(def side 3)\n";
        const LOADING_INPUT: &str = "(use shapes side)\n";

        fn new() -> Self {
            Self::in_directory(Path::new(""))
        }

        fn in_directory(directory: &Path) -> Self {
            Self {
                sources: SourceMap::default(),
                work: StageWork::default(),
                output: RecordedOutput::default(),
                file: ProgramFile::beside_itself(directory.join(Self::MODULE)),
            }
        }

        fn expansion(&mut self) -> Expansion<'_> {
            Expansion::new(&mut self.sources, &mut self.work, &self.output)
        }

        fn added(&mut self, text: &str) -> SourceId {
            self.sources.add(Self::MODULE, text)
        }

        fn module(&mut self, texts: &[&str]) -> ModuleOfSources {
            ModuleOfSources {
                file: self.file.clone(),
                sources: texts.iter().map(|text| self.added(text)).collect(),
            }
        }

        fn module_with_an_input_after_the_prelude(
            &mut self,
            entry: &str,
            input: &str,
        ) -> ModuleOfSources {
            let entry = self.module(&[entry]);
            self.load_the_prelude_after(&entry);
            let input = self.added(input);
            ModuleOfSources {
                file: entry.file,
                sources: vec![entry.sources[0], input],
            }
        }

        fn load_the_prelude_after(&mut self, entry: &ModuleOfSources) {
            self.expansion()
                .classify(entry, Span::new(entry.anchor(), 0..0));
        }

        fn span_of(&self, source: SourceId, form: &str) -> Span {
            let start = self
                .sources
                .file(source)
                .text()
                .find(form)
                .expect("the form is in the text");
            Span::new(source, start..start + form.len())
        }

        fn written_module(directory: &TemporaryDirectory) -> PathBuf {
            let path = directory
                .path()
                .join(format!("{}.knv", Self::LOADED_MODULE));
            fs::write(&path, Self::LOADED_TEXT).expect("the scratch directory is writable");
            path
        }
    }

    #[test]
    fn expands_an_entry_and_records_its_lowered_forms() {
        let mut fixture = ExpansionFixture::new();
        let entry = fixture.added(ExpansionFixture::DEFINITIONS);
        let file = fixture.file.clone();
        let program = fixture.expansion().expand(&file, entry);
        assert!(program.is_ok());
        assert!(fixture.work.lowered_forms.is_some_and(|forms| forms > 0));
    }

    #[test]
    fn reports_an_entry_that_does_not_expand() {
        let mut fixture = ExpansionFixture::new();
        let entry = fixture.added(ExpansionFixture::UNBOUND);
        let file = fixture.file.clone();
        let status = fixture.expansion().expand(&file, entry).err();
        assert_eq!(status, Some(ExitStatus::ProgramErrors));
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn finishes_a_session_on_a_module() {
        let mut fixture = ExpansionFixture::new();
        let module = fixture.module(&[ExpansionFixture::DEFINITIONS]);
        let program = fixture
            .expansion()
            .within_session(&module, |session| session.finish());
        assert!(program.is_ok());
    }

    #[test]
    fn reports_a_session_that_does_not_finish() {
        let mut fixture = ExpansionFixture::new();
        let module = fixture.module(&[ExpansionFixture::UNBOUND]);
        let status = fixture
            .expansion()
            .within_session(&module, |session| session.finish())
            .err();
        assert_eq!(status, Some(ExitStatus::ProgramErrors));
        assert_eq!(fixture.output.writes(), ["1 diagnostics"]);
    }

    #[test]
    fn evaluates_the_forms_a_request_names() {
        let mut fixture = ExpansionFixture::new();
        let module = fixture.module(&[ExpansionFixture::DEFINITIONS]);
        let evaluated = fixture.span_of(module.anchor(), ExpansionFixture::SECOND_DEFINITION);
        let request = EvaluationRequest {
            module,
            evaluated: vec![evaluated],
        };
        let defined = fixture
            .expansion()
            .within_evaluation(&request, |session| Ok(session.defines(evaluated)));
        assert_eq!(defined.ok(), Some(true));
    }

    #[test]
    fn classifies_a_definition() {
        let mut fixture = ExpansionFixture::new();
        let module = fixture.module(&[ExpansionFixture::DEFINITIONS]);
        let form = fixture.span_of(module.anchor(), ExpansionFixture::SECOND_DEFINITION);
        let classified = fixture.expansion().classify(&module, form);
        assert!(matches!(classified, ClassifiedForm::Definition(_)));
    }

    #[test]
    fn surveys_the_errors_of_a_module_that_does_not_expand() {
        let mut fixture = ExpansionFixture::new();
        let module = fixture.module(&[ExpansionFixture::UNBOUND]);
        let survey = fixture.expansion().survey(&module, &[]);
        assert_eq!(survey.diagnostics().len(), 1);
    }

    #[test]
    fn surveys_the_type_errors_of_a_module_that_expands() {
        let mut fixture = ExpansionFixture::new();
        let module = fixture.module(&[ExpansionFixture::MISTYPED]);
        let survey = fixture.expansion().survey(&module, &[]);
        assert!(survey.diagnostics().has_errors());
    }

    #[test]
    fn surveys_the_warnings_of_a_module_that_checks() {
        let mut fixture = ExpansionFixture::new();
        let module = fixture.module(&[ExpansionFixture::UNREACHABLE_ARM]);
        let survey = fixture.expansion().survey(&module, &[]);
        assert_eq!(survey.diagnostics().len(), 1);
        assert!(!survey.diagnostics().has_errors());
    }

    #[test]
    fn surveys_the_loader_and_the_names_of_each_input() {
        let directory = TemporaryDirectory::create().expect("a scratch directory is created");
        let loaded = ExpansionFixture::written_module(&directory);
        let mut fixture = ExpansionFixture::in_directory(directory.path());
        let module = fixture.module_with_an_input_after_the_prelude(
            ExpansionFixture::DEFINITIONS,
            ExpansionFixture::LOADING_INPUT,
        );
        let definition = fixture.span_of(module.sources[0], ExpansionFixture::FIRST_DEFINITION);
        let loading = fixture.span_of(
            module.sources[1],
            ExpansionFixture::LOADING_INPUT.trim_end(),
        );
        let survey = fixture.expansion().survey(&module, &[definition, loading]);
        let loaders: Vec<(String, SourceId)> = survey
            .loaders()
            .iter()
            .map(|loader| (loader.file.clone(), loader.input))
            .collect();
        assert_eq!(loaders, [(loaded.display().to_string(), module.sources[1])]);
        let (names, _) = survey.into_names_and_loaders();
        let names: Vec<Vec<String>> = names
            .iter()
            .map(|input| input.iter().map(|bound| bound.name.to_string()).collect())
            .collect();
        assert_eq!(names, [vec!["width"], vec![]]);
    }
}
