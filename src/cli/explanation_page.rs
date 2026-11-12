use super::exit_status::ExitStatus;
use super::standard_output::StandardOutput;
use crate::diagnostic::Explanation;

pub struct ExplanationPage {
    explanation: Explanation,
}

impl ExplanationPage {
    const UNWRITTEN: &'static str = "No explanation yet.";

    pub fn new(explanation: Explanation) -> Self {
        Self { explanation }
    }

    pub fn print(&self) -> ExitStatus {
        StandardOutput::locked().write(self.text())
    }

    pub fn markdown(&self) -> Option<String> {
        let body = self.explanation.text()?.trim_end();
        Some(format!("# {}\n\n{body}\n", self.header()))
    }

    pub fn code(&self) -> &str {
        self.explanation.code().as_str()
    }

    pub fn display_name(&self) -> String {
        self.explanation.display_name()
    }

    pub fn is_written(&self) -> bool {
        self.explanation.text().is_some()
    }

    fn text(&self) -> String {
        format!("{}\n\n{}\n", self.header(), self.body())
    }

    fn header(&self) -> String {
        format!("{}: {}", self.code(), self.display_name())
    }

    fn body(&self) -> &'static str {
        self.explanation
            .text()
            .map_or(Self::UNWRITTEN, str::trim_end)
    }
}
