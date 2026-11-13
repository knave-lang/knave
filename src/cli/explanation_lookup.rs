use super::doc::DocLookup;
use super::exit_status::ExitStatus;
use super::terminal::Terminal;
use crate::diagnostic::{CodeStage, Explanation};

pub struct ExplanationLookup<'t> {
    lookup: DocLookup<'t>,
}

impl<'t> ExplanationLookup<'t> {
    pub fn new(terminal: &'t mut Terminal) -> Self {
        Self {
            lookup: DocLookup::new(terminal),
        }
    }

    pub fn docstring(&mut self, explanation: &Explanation) -> Result<Option<String>, ExitStatus> {
        match Self::docstring_explained_lint(explanation) {
            Some(lint) => self.lookup.lint_explanation(&lint),
            None => Ok(None),
        }
    }

    fn docstring_explained_lint(explanation: &Explanation) -> Option<String> {
        if explanation.text().is_some() {
            return None;
        }
        CodeStage::LINTS.name_of(explanation.code())
    }
}
