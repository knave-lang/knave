use super::doc::DocLookup;
use super::exit_status::ExitStatus;
use super::terminal::Terminal;
use crate::diagnostic::{CodeStage, Explanation};

pub struct ExplanationLookup<'t> {
    lookup: DocLookup<'t>,
}

impl<'t> ExplanationLookup<'t> {
    const EVERY_CODE_EXPLAINED: &'static str =
        "the explanation tests give every code a Markdown file or a lint docstring";

    pub fn new(terminal: &'t mut Terminal) -> Self {
        Self {
            lookup: DocLookup::new(terminal),
        }
    }

    pub fn text(&mut self, explanation: &Explanation) -> Result<String, ExitStatus> {
        match explanation.text() {
            Some(text) => Ok(text.to_string()),
            None => self.lint_docstring(explanation),
        }
    }

    fn lint_docstring(&mut self, explanation: &Explanation) -> Result<String, ExitStatus> {
        let lint = CodeStage::LINTS
            .name_of(explanation.code())
            .expect(Self::EVERY_CODE_EXPLAINED);
        let docstring = self.lookup.lint_explanation(&lint)?;
        Ok(docstring.expect(Self::EVERY_CODE_EXPLAINED))
    }
}
