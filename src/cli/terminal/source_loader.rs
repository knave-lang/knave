use std::path::Path;

use crate::cli::diagnostic_output::{DiagnosticOutput, UnreadableFile};
use crate::cli::exit_status::ExitStatus;
use crate::diagnostic::Diagnostics;
use crate::modules::ModuleSources;
use crate::project::{Project, ProjectReader};
use crate::reader::{Form, Reader};
use crate::source::{SourceId, SourceMap, Span, TextPosition};

pub struct SourceLoader<'a> {
    sources: &'a mut SourceMap,
    output: &'a dyn DiagnosticOutput,
}

impl<'a> SourceLoader<'a> {
    pub fn new(sources: &'a mut SourceMap, output: &'a dyn DiagnosticOutput) -> Self {
        Self { sources, output }
    }

    pub fn load(&mut self, path: &Path) -> Result<SourceId, ExitStatus> {
        match ModuleSources::read_file(path) {
            Ok(text) => Ok(self.add(path, text)),
            Err(error) => Err(self.report_unreadable(&UnreadableFile::new(path, &error))),
        }
    }

    pub fn add(&mut self, path: &Path, text: String) -> SourceId {
        self.sources.add(path.display().to_string(), text)
    }

    pub fn add_named(&mut self, name: &str, text: &str) -> SourceId {
        self.sources.add(name, text)
    }

    pub fn add_inside(
        &mut self,
        enclosing: SourceId,
        position: TextPosition,
        text: &str,
    ) -> Option<SourceId> {
        self.sources.add_inside(enclosing, position, text)
    }

    pub fn read(&self, source: SourceId) -> Result<Vec<Form>, ExitStatus> {
        Reader::new(self.sources.file(source))
            .read()
            .map_err(|diagnostics| self.report(&diagnostics))
    }

    pub fn read_project(&self, source: SourceId) -> Result<Project, ExitStatus> {
        let forms = self.read(source)?;
        let whole_file = Span::new(source, 0..self.sources.file(source).text().len());
        ProjectReader::read(&forms, whole_file).map_err(|diagnostics| self.report(&diagnostics))
    }

    fn report(&self, diagnostics: &Diagnostics) -> ExitStatus {
        self.output.report(self.sources, diagnostics)
    }

    fn report_unreadable(&self, file: &UnreadableFile) -> ExitStatus {
        self.output.write_error(&file.to_string());
        UnreadableFile::EXIT_STATUS
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::Path;

    use super::SourceLoader;
    use crate::cli::diagnostic_output::DiagnosticOutput;
    use crate::cli::exit_status::ExitStatus;
    use crate::diagnostic::Diagnostics;
    use crate::source::{SourceId, SourceMap, TextPosition};

    #[derive(Default)]
    struct RecordedOutput {
        reports: RefCell<Vec<String>>,
    }

    impl DiagnosticOutput for RecordedOutput {
        fn write(&self, _: &SourceMap, diagnostics: &Diagnostics) {
            self.reports
                .borrow_mut()
                .push(format!("{} diagnostics", diagnostics.len()));
        }

        fn write_note(&self, _: &SourceMap, message: &str) {
            self.reports.borrow_mut().push(message.to_string());
        }

        fn write_error(&self, message: &str) {
            self.reports.borrow_mut().push(message.to_string());
        }
    }

    #[derive(Default)]
    struct LoaderFixture {
        sources: SourceMap,
        output: RecordedOutput,
    }

    impl LoaderFixture {
        const MODULE: &str = "geometry.knv";
        const MISSING: &str = "no-such-directory/missing.knv";

        fn loader(&mut self) -> SourceLoader<'_> {
            SourceLoader::new(&mut self.sources, &self.output)
        }

        fn added(&mut self, text: &str) -> SourceId {
            self.loader().add(Path::new(Self::MODULE), text.to_string())
        }

        fn reports(&self) -> Vec<String> {
            self.output.reports.borrow().clone()
        }
    }

    #[test]
    fn reads_the_forms_of_an_added_text() {
        let mut fixture = LoaderFixture::default();
        let source = fixture.added("(area 2 3) (area 4 5)");
        let forms = fixture.loader().read(source).expect("the text reads");
        assert_eq!(forms.len(), 2);
    }

    #[test]
    fn names_an_added_text_by_its_path() {
        let mut fixture = LoaderFixture::default();
        let source = fixture.added("(area 2 3)");
        assert_eq!(fixture.sources.file(source).name(), LoaderFixture::MODULE);
    }

    #[test]
    fn names_a_named_text_by_its_name() {
        let mut fixture = LoaderFixture::default();
        let source = fixture.loader().add_named("<input 1>", "(area 2 3)");
        assert_eq!(fixture.sources.file(source).name(), "<input 1>");
    }

    #[test]
    fn reports_a_text_that_does_not_read() {
        let mut fixture = LoaderFixture::default();
        let source = fixture.added("(area 2 3");
        let status = fixture.loader().read(source);
        assert_eq!(status.err(), Some(ExitStatus::ProgramErrors));
        assert_eq!(fixture.reports(), ["1 diagnostics"]);
    }

    #[test]
    fn reports_a_missing_file_through_its_output() {
        let mut fixture = LoaderFixture::default();
        let status = fixture.loader().load(Path::new(LoaderFixture::MISSING));
        assert_eq!(status.err(), Some(ExitStatus::UsageError));
        let [report] = fixture.reports().try_into().expect("one report");
        assert!(report.starts_with("cannot read `no-such-directory/missing.knv`: "));
    }

    #[test]
    fn adds_an_excerpt_found_at_its_position_under_its_module_name() {
        let mut fixture = LoaderFixture::default();
        let module = fixture.added("(area 2 3)\n(area 4 5)\n");
        let excerpt = fixture
            .loader()
            .add_inside(module, TextPosition::new(2, 1), "(area 4 5)")
            .expect("the excerpt is at its position");
        assert_eq!(fixture.sources.file(excerpt).name(), LoaderFixture::MODULE);
    }

    #[test]
    fn refuses_an_excerpt_missing_from_its_position() {
        let mut fixture = LoaderFixture::default();
        let module = fixture.added("(area 2 3)\n");
        let excerpt = fixture
            .loader()
            .add_inside(module, TextPosition::new(1, 1), "(area 4 5)");
        assert!(excerpt.is_none());
    }

    #[test]
    fn reads_a_project_from_its_project_form() {
        let mut fixture = LoaderFixture::default();
        let source = fixture.added("(defproject app)");
        let project = fixture
            .loader()
            .read_project(source)
            .expect("the project reads");
        assert_eq!(project.name, "app");
    }

    #[test]
    fn reports_a_project_file_without_a_project_form() {
        let mut fixture = LoaderFixture::default();
        let source = fixture.added("(area 2 3)");
        let status = fixture.loader().read_project(source);
        assert_eq!(status.err(), Some(ExitStatus::ProgramErrors));
        assert_eq!(fixture.reports(), ["1 diagnostics"]);
    }
}
