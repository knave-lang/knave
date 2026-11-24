use super::level_words::LevelWords;
use super::lint_level::LintLevel;
use crate::diagnostic::{CodeStage, DiagnosticCode};
use crate::reader::Form;
use crate::source::Span;

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LintDefaults {
    levels: Vec<(DiagnosticCode, LintLevel)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LintDefaultProblem {
    Malformed(Span),
    Repeated(Span, String),
    UnknownLint(Span, String),
    UnknownLevel(Span, String),
}

impl LintDefaultProblem {
    pub fn site(&self) -> Span {
        match self {
            Self::Malformed(site)
            | Self::Repeated(site, _)
            | Self::UnknownLint(site, _)
            | Self::UnknownLevel(site, _) => *site,
        }
    }

    pub fn fault(&self) -> String {
        match self {
            Self::Malformed(_) => "needs one lint name and one level name".to_string(),
            Self::Repeated(_, lint) => format!("gives `{lint}` a second default level"),
            Self::UnknownLint(_, lint) => format!("names `{lint}`, which is no lint"),
            Self::UnknownLevel(_, level) => format!("names `{level}`, which is no lint level"),
        }
    }
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
            return Err(Self::repeated(note, &code));
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
        let (lint_name, level_name) = Self::name_of(lint)
            .zip(Self::name_of(level))
            .ok_or_else(|| Self::malformed(note))?;
        let code = CodeStage::LINTS
            .code_named(lint_name)
            .ok_or_else(|| LintDefaultProblem::UnknownLint(lint.span(), lint_name.to_string()))?;
        let level = words.level_named(level_name).ok_or_else(|| {
            LintDefaultProblem::UnknownLevel(level.span(), level_name.to_string())
        })?;
        Ok((code, level))
    }

    fn repeated(note: &Form, code: &DiagnosticCode) -> LintDefaultProblem {
        let lint = CodeStage::LINTS
            .name_of(code)
            .expect("a lint default names a lint");
        LintDefaultProblem::Repeated(Self::site_of(note), lint)
    }

    fn malformed(note: &Form) -> LintDefaultProblem {
        LintDefaultProblem::Malformed(Self::site_of(note))
    }

    fn site_of(note: &Form) -> Span {
        note.as_list()
            .and_then(|parts| parts.first())
            .unwrap_or(note)
            .span()
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

    struct DefaultsFixture {
        sources: SourceMap,
        read: Result<LintDefaults, LintDefaultProblem>,
    }

    impl DefaultsFixture {
        fn read(text: &str) -> Self {
            let mut sources = SourceMap::default();
            let source = sources.add("notes", text);
            let notes: Vec<Form> = Reader::new(sources.file(source))
                .read()
                .expect("the notes read");
            let read = LintDefaults::read(&notes, &Self::words());
            Self { sources, read }
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

        fn defaults(self) -> LintDefaults {
            self.read.expect("every note names a lint and a level")
        }

        fn refusal(&self) -> (&str, String) {
            let Err(problem) = &self.read else {
                panic!("the notes are refused");
            };
            let site = problem.site();
            let text = &self.sources.file(site.source()).text()[site.range()];
            (text, problem.fault())
        }
    }

    #[test]
    fn each_note_gives_its_lint_a_default_level() {
        let defaults =
            DefaultsFixture::read("(needless-type hush) (unused-binding stop)").defaults();
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
    fn a_default_of_an_unknown_lint_is_refused_at_the_lint() {
        let fixture = DefaultsFixture::read("(tidy-names hush)");
        assert_eq!(
            fixture.refusal(),
            (
                "tidy-names",
                "names `tidy-names`, which is no lint".to_string()
            )
        );
    }

    #[test]
    fn a_default_of_an_unknown_level_is_refused_at_the_level() {
        let fixture = DefaultsFixture::read("(needless-type allow)");
        assert_eq!(
            fixture.refusal(),
            ("allow", "names `allow`, which is no lint level".to_string())
        );
    }

    #[test]
    fn a_second_default_of_one_lint_is_refused_at_the_second() {
        let fixture = DefaultsFixture::read("(needless-type hush) (needless-type note)");
        let (site, fault) = fixture.refusal();
        assert_eq!(fault, "gives `needless-type` a second default level");
        assert_eq!(site, "needless-type");
        let Err(problem) = &fixture.read else {
            panic!("the second default is refused");
        };
        assert!(problem.site().range().start > "(needless-type hush)".len());
    }

    #[test]
    fn a_default_without_a_level_is_refused() {
        let fixture = DefaultsFixture::read("(needless-type)");
        assert_eq!(
            fixture.refusal(),
            (
                "needless-type",
                "needs one lint name and one level name".to_string()
            )
        );
    }

    #[test]
    fn a_default_that_names_its_level_by_a_string_is_refused() {
        let fixture = DefaultsFixture::read("(needless-type \"hush\")");
        assert_eq!(
            fixture.refusal().1,
            "needs one lint name and one level name"
        );
    }

    #[test]
    fn an_empty_default_is_refused_at_itself() {
        let fixture = DefaultsFixture::read("()");
        assert_eq!(fixture.refusal().0, "()");
    }
}
