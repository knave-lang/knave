use super::level_words::LevelWords;
use super::lint_level::LintLevel;
use super::role_book::RoleBook;
use super::role_problem::RoleProblem;
use crate::reader::Form;

pub(crate) struct LevelKeys;

impl LevelKeys {
    pub(crate) const NOTE: &'static str = "lint-levels";

    pub(crate) fn read(notes: &[Form]) -> Result<LevelWords, RoleProblem> {
        let mut book = RoleBook::of(notes, |_| Ok(()))?;
        let words = LintLevel::ALL
            .into_iter()
            .map(|level| Ok((level, book.taken(level.key())?)))
            .collect::<Result<Vec<_>, RoleProblem>>()?;
        book.finish()?;
        Ok(LevelWords::new(words))
    }
}

#[cfg(test)]
mod tests {
    use super::LevelKeys;
    use crate::project::LintLevel;
    use crate::project::level_words::LevelWords;
    use crate::project::role_problem::RoleProblem;
    use crate::reader::{Form, Reader};
    use crate::source::SourceMap;

    struct KeysFixture;

    impl KeysFixture {
        const SHUFFLED: &str = "(stop error-level) (hush silent-level) (note warning-level)";

        fn read(text: &str) -> Result<LevelWords, RoleProblem> {
            let mut sources = SourceMap::default();
            let source = sources.add("notes", text);
            let notes: Vec<Form> = Reader::new(sources.file(source))
                .read()
                .expect("the notes read");
            LevelKeys::read(&notes)
        }

        fn with(extra: &str) -> Result<LevelWords, RoleProblem> {
            Self::read(&format!("{} {extra}", Self::SHUFFLED))
        }
    }

    #[test]
    fn each_word_takes_the_level_of_its_key_whatever_its_place() {
        let words = KeysFixture::read(KeysFixture::SHUFFLED).expect("every key is stated once");
        assert_eq!(words.level_named("hush"), Some(LintLevel::Allow));
        assert_eq!(words.level_named("note"), Some(LintLevel::Warn));
        assert_eq!(words.level_named("stop"), Some(LintLevel::Deny));
    }

    #[test]
    fn a_missing_key_fails() {
        let failure = KeysFixture::read("(stop error-level) (hush silent-level)");
        assert_eq!(failure, Err(RoleProblem::Missing("warning-level")));
    }

    #[test]
    fn a_repeated_key_fails() {
        let failure = KeysFixture::with("(halt error-level)");
        assert_eq!(
            failure,
            Err(RoleProblem::Repeated("error-level".to_string()))
        );
    }

    #[test]
    fn an_unknown_key_fails() {
        let failure = KeysFixture::with("(shout loud-level)");
        assert_eq!(failure, Err(RoleProblem::Unknown("loud-level".to_string())));
    }

    #[test]
    fn a_word_with_two_keys_fails() {
        let failure = KeysFixture::read("(stop error-level) (stop silent-level)");
        assert_eq!(failure, Err(RoleProblem::TwoRoles("stop".to_string())));
    }
}
