use super::exit_status::ExitStatus;
use super::standard_output::StandardOutput;
use crate::diagnostic::Explanation;

pub struct ExplanationPage {
    explanation: Explanation,
    text: String,
}

impl ExplanationPage {
    pub fn new(explanation: Explanation, text: String) -> Self {
        Self { explanation, text }
    }

    pub fn print(&self) -> ExitStatus {
        StandardOutput::locked().write(format!("{}\n\n{}\n", self.header(), self.body()))
    }

    pub fn markdown(&self) -> String {
        format!("# {}\n\n{}\n", self.header(), self.body())
    }

    pub fn code(&self) -> &str {
        self.explanation.code().as_str()
    }

    pub fn display_name(&self) -> String {
        self.explanation.display_name()
    }

    fn header(&self) -> String {
        format!("{}: {}", self.code(), self.display_name())
    }

    fn body(&self) -> &str {
        self.text.trim_end()
    }
}
