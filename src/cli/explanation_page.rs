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

    fn text(&self) -> String {
        format!("{}\n\n{}\n", self.header(), self.body())
    }

    fn header(&self) -> String {
        format!(
            "{}: {}",
            self.explanation.code().as_str(),
            self.explanation.display_name()
        )
    }

    fn body(&self) -> &'static str {
        self.explanation
            .text()
            .map_or(Self::UNWRITTEN, str::trim_end)
    }
}
