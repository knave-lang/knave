use std::rc::Rc;

use super::level_words::LevelWords;
use super::lint_defaults::{LintDefaultProblem, LintDefaults};
use super::lint_level::LintLevel;
use crate::diagnostic::DiagnosticCode;
use crate::reader::Form;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LintVocabulary {
    words: LevelWords,
    defaults: LintDefaults,
}

impl LintVocabulary {
    pub(super) fn new(words: LevelWords) -> Self {
        Self {
            words,
            defaults: LintDefaults::default(),
        }
    }

    pub fn level_of(&self, code: &DiagnosticCode, written: Option<LintLevel>) -> Option<LintLevel> {
        written.or_else(|| self.defaults.level_of(code))
    }

    pub(super) fn with_defaults(self, notes: &[Form]) -> Result<Self, LintDefaultProblem> {
        let defaults = LintDefaults::read(notes, &self.words)?;
        Ok(Self { defaults, ..self })
    }

    pub(super) fn level_named(&self, word: &str) -> Option<LintLevel> {
        self.words.level_named(word)
    }

    pub(super) fn level_words(&self) -> Vec<Rc<str>> {
        self.words.words()
    }
}
