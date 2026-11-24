use super::level_words::LevelWords;
use super::lint_level::LintLevel;
use crate::diagnostic::{CodeStage, DiagnosticCode};
use crate::reader::Form;

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LintDefaults {
    levels: Vec<(DiagnosticCode, LintLevel)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LintDefaultProblem {
    Malformed(String),
    Repeated(String),
    UnknownLint(String),
    UnknownLevel(String),
}

impl LintDefaults {
    pub(crate) const NOTE: &'static str = "lint-defaults";

    pub(super) fn read(notes: &[Form], words: &LevelWords) -> Result<Self, LintDefaultProblem> {
        let mut defaults = Self::default();
        for note in notes {
            defaults.add(note, words)?;
        }
        Ok(defaults)
    }

    pub(super) fn level_of(&self, code: &DiagnosticCode) -> Option<LintLevel> {
        self.levels
            .iter()
            .find(|(default_code, _)| default_code == code)
            .map(|(_, level)| *level)
    }

    fn add(&mut self, note: &Form, words: &LevelWords) -> Result<(), LintDefaultProblem> {
        let (code, level) = Self::entry_of(note, words)?;
        if self.level_of(&code).is_some() {
            return Err(LintDefaultProblem::Repeated(note.to_string()));
        }
        self.levels.push((code, level));
        Ok(())
    }

    fn entry_of(
        note: &Form,
        words: &LevelWords,
    ) -> Result<(DiagnosticCode, LintLevel), LintDefaultProblem> {
        let Some([lint, level]) = note.as_list() else {
            return Err(Self::malformed(note));
        };
        let (lint, level) = Self::name_of(lint)
            .zip(Self::name_of(level))
            .ok_or_else(|| Self::malformed(note))?;
        let code = CodeStage::LINTS
            .code_named(lint)
            .ok_or_else(|| LintDefaultProblem::UnknownLint(lint.to_string()))?;
        let level = words
            .level_named(level)
            .ok_or_else(|| LintDefaultProblem::UnknownLevel(level.to_string()))?;
        Ok((code, level))
    }

    fn malformed(note: &Form) -> LintDefaultProblem {
        LintDefaultProblem::Malformed(note.to_string())
    }

    fn name_of(form: &Form) -> Option<&str> {
        form.as_symbol().map(|symbol| symbol.name())
    }
}

#[cfg(test)]
mod tests {
    use super::{LintDefaultProblem, LintDefaults};
    use crate::diagnostic::DiagnosticCode;
    use crate::project::LintLevel;
    use crate::project::level_words::LevelWords;
    use crate::reader::{Form, Reader};
    use crate::source::SourceMap;

    struct DefaultsFixture;

    impl DefaultsFixture {
        fn read(text: &str) -> Result<LintDefaults, LintDefaultProblem> {
            let mut sources = SourceMap::default();
            let source = sources.add("notes", text);
            let notes: Vec<Form> = Reader::new(sources.file(source))
                .read()
                .expect("the notes read");
            LintDefaults::read(&notes, &Self::words())
        }

        fn words() -> LevelWords {
            LevelWords::new(
                LintLevel::ALL
                    .into_iter()
                    .zip(["hush", "note", "stop"])
                    .map(|(level, word)| (level, word.to_string()))
                    .collect(),
            )
        }
    }

    #[test]
    fn each_note_gives_its_lint_a_default_level() {
        let defaults = DefaultsFixture::read("(needless-type hush) (unused-binding stop)")
            .expect("every note names a lint and a level");
        assert_eq!(
            defaults.level_of(&DiagnosticCode::NEEDLESS_TYPE),
            Some(LintLevel::Allow)
        );
        assert_eq!(
            defaults.level_of(&DiagnosticCode::UNUSED_BINDING),
            Some(LintLevel::Deny)
        );
        assert_eq!(defaults.level_of(&DiagnosticCode::UNUSED_IMPORT), None);
    }

    #[test]
    fn a_default_of_an_unknown_lint_is_refused() {
        let failure = DefaultsFixture::read("(tidy-names hush)");
        assert_eq!(
            failure,
            Err(LintDefaultProblem::UnknownLint("tidy-names".to_string()))
        );
    }

    #[test]
    fn a_default_of_an_unknown_level_is_refused() {
        let failure = DefaultsFixture::read("(needless-type allow)");
        assert_eq!(
            failure,
            Err(LintDefaultProblem::UnknownLevel("allow".to_string()))
        );
    }

    #[test]
    fn a_second_default_of_one_lint_is_refused() {
        let failure = DefaultsFixture::read("(needless-type hush) (needless-type note)");
        assert_eq!(
            failure,
            Err(LintDefaultProblem::Repeated(
                "(needless-type note)".to_string()
            ))
        );
    }

    #[test]
    fn a_default_without_a_level_is_refused() {
        assert_eq!(
            DefaultsFixture::read("(needless-type)"),
            Err(LintDefaultProblem::Malformed("(needless-type)".to_string()))
        );
    }

    #[test]
    fn a_default_that_names_its_level_by_a_string_is_refused() {
        assert_eq!(
            DefaultsFixture::read("(needless-type \"hush\")"),
            Err(LintDefaultProblem::Malformed(
                "(needless-type \"hush\")".to_string()
            ))
        );
    }
}
