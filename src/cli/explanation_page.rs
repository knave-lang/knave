use super::exit_status::ExitStatus;
use super::standard_output::StandardOutput;
use crate::diagnostic::Explanation;

pub struct ExplanationPage {
    explanation: Explanation,
    docstring: Option<String>,
}

impl ExplanationPage {
    const UNWRITTEN: &'static str = "No explanation yet.";

    pub fn new(explanation: Explanation) -> Self {
        Self {
            explanation,
            docstring: None,
        }
    }

    pub fn with_docstring(self, docstring: Option<String>) -> Self {
        Self { docstring, ..self }
    }

    pub fn print(&self) -> ExitStatus {
        StandardOutput::locked().write(self.text())
    }

    pub fn markdown(&self) -> Option<String> {
        let body = self.written()?.trim_end();
        Some(format!("# {}\n\n{body}\n", self.header()))
    }

    pub fn code(&self) -> &str {
        self.explanation.code().as_str()
    }

    pub fn display_name(&self) -> String {
        self.explanation.display_name()
    }

    pub fn is_written(&self) -> bool {
        self.written().is_some()
    }

    fn written(&self) -> Option<&str> {
        self.explanation.text().or(self.docstring.as_deref())
    }

    fn text(&self) -> String {
        format!("{}\n\n{}\n", self.header(), self.body())
    }

    fn header(&self) -> String {
        format!("{}: {}", self.code(), self.display_name())
    }

    fn body(&self) -> &str {
        self.written().map_or(Self::UNWRITTEN, str::trim_end)
    }
}
