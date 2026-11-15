use std::rc::Rc;

use super::lint_level::LintLevel;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LevelWords {
    words: Vec<(LintLevel, String)>,
}

impl LevelWords {
    pub(super) fn new(words: Vec<(LintLevel, String)>) -> Self {
        Self { words }
    }

    pub(super) fn level_named(&self, word: &str) -> Option<LintLevel> {
        self.words
            .iter()
            .find(|(_, level_word)| level_word == word)
            .map(|(level, _)| *level)
    }

    pub(super) fn words(&self) -> Vec<Rc<str>> {
        self.words
            .iter()
            .map(|(_, word)| Rc::from(word.as_str()))
            .collect()
    }
}
