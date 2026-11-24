use std::rc::Rc;

use super::level_words::LevelWords;
use super::lint_level::LintLevel;
use super::written_entries::WrittenEntries;
use crate::diagnostic::{CodeStage, DiagnosticCode, SimilarName};
use crate::reader::Form;
use crate::source::Span;

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LintDefaults {
    levels: Vec<(DiagnosticCode, LintLevel)>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LintDefaultProblem {
    Malformed(Span),
    Repeated(Span, String, Span),
    UnknownLint(Span, String, Option<Rc<str>>),
    UnknownLevel(Span, String, Option<Rc<str>>),
}

impl LintDefaultProblem {
    pub(crate) fn message(&self, written: &impl WrittenEntries) -> String {
        let refusal = format!("{} {}", written.quoted_call(self.site()), self.fault());
        match self.first_site() {
            Some(first) => format!("{refusal}; the first is at {}", written.location(first)),
            None => refusal,
        }
    }

    fn site(&self) -> Span {
        match self {
            Self::Malformed(site)
            | Self::Repeated(site, ..)
            | Self::UnknownLint(site, ..)
            | Self::UnknownLevel(site, ..) => *site,
        }
    }

    fn first_site(&self) -> Option<Span> {
        match self {
            Self::Repeated(_, _, first) => Some(*first),
            _ => None,
        }
    }

    fn fault(&self) -> String {
        match self {
            Self::Malformed(_) => "needs one lint name and one level name".to_string(),
            Self::Repeated(_, lint, _) => format!("gives `{lint}` a second default level"),
            Self::UnknownLint(_, lint, closest) => {
                let suggestion = Self::suggestion("lint", closest.as_deref());
                format!("names `{lint}`, which is no lint{suggestion}")
            }
            Self::UnknownLevel(_, level, closest) => {
                let suggestion = Self::suggestion("level", closest.as_deref());
                format!("names `{level}`, which is no lint level{suggestion}")
            }
        }
    }

    fn suggestion(kind: &str, closest: Option<&str>) -> String {
        closest.map_or_else(String::new, |name| {
            format!("; the closest {kind} is `{name}`")
        })
    }
}

impl LintDefaults {
    pub(crate) const NOTE: &'static str = "lint-defaults";

    pub(super) fn read(notes: &[Form], words: &LevelWords) -> Result<Self, LintDefaultProblem> {
        let mut defaults = Self::default();
        for (position, note) in notes.iter().enumerate() {
            defaults.add(note, &notes[..position], words)?;
        }
        Ok(defaults)
    }

    pub(super) fn level_of(&self, code: &DiagnosticCode) -> Option<LintLevel> {
        self.levels
            .iter()
            .find(|(default_code, _)| default_code == code)
            .map(|(_, level)| *level)
    }

    fn add(
        &mut self,
        note: &Form,
        earlier: &[Form],
        words: &LevelWords,
    ) -> Result<(), LintDefaultProblem> {
        let (code, level) = Self::entry_of(note, words)?;
        if self.level_of(&code).is_some() {
            return Err(Self::repeated(note, earlier, &code));
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
            .ok_or_else(|| Self::unknown_lint(lint, lint_name))?;
        let level = words
            .level_named(level_name)
            .ok_or_else(|| Self::unknown_level(level, level_name, words))?;
        Ok((code, level))
    }

    fn unknown_lint(lint: &Form, name: &str) -> LintDefaultProblem {
        let closest = SimilarName::closest(name, &CodeStage::LINTS.names());
        LintDefaultProblem::UnknownLint(lint.span(), name.to_string(), closest)
    }

    fn unknown_level(level: &Form, name: &str, words: &LevelWords) -> LintDefaultProblem {
        let closest = SimilarName::closest(name, &words.words());
        LintDefaultProblem::UnknownLevel(level.span(), name.to_string(), closest)
    }

    fn repeated(note: &Form, earlier: &[Form], code: &DiagnosticCode) -> LintDefaultProblem {
        let lint = CodeStage::LINTS
            .name_of(code)
            .expect("a lint default names a lint");
        let first = earlier
            .iter()
            .find(|entry| Self::lint_name_of(entry) == Some(lint.as_str()))
            .expect("a repeated lint has a first default");
        LintDefaultProblem::Repeated(Self::site_of(note), lint, Self::site_of(first))
    }

    fn lint_name_of(entry: &Form) -> Option<&str> {
        entry.as_list()?.first().and_then(Self::name_of)
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
    use crate::project::level_words::LevelWords;
    use crate::project::{LintLevel, WrittenEntries};
    use crate::reader::{Form, Reader};
    use crate::source::{SourceMap, Span};

    struct DefaultsFixture {
        sources: SourceMap,
        notes: Vec<Form>,
        read: Result<LintDefaults, LintDefaultProblem>,
    }

    impl DefaultsFixture {
        const FILE: &str = "lint.knv";

        fn read(text: &str) -> Self {
            let mut sources = SourceMap::default();
            let source = sources.add(Self::FILE, text);
            let notes: Vec<Form> = Reader::new(sources.file(source))
                .read()
                .expect("the notes read");
            let read = LintDefaults::read(&notes, &Self::words());
            Self {
                sources,
                notes,
                read,
            }
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
            let problem = self.problem();
            (self.text_at(problem.site()), problem.fault())
        }

        fn message(&self) -> String {
            self.problem().message(self)
        }

        fn problem(&self) -> &LintDefaultProblem {
            let Err(problem) = &self.read else {
                panic!("the notes are refused");
            };
            problem
        }

        fn text_at(&self, site: Span) -> &str {
            &self.sources.file(site.source()).text()[site.range()]
        }
    }

    impl WrittenEntries for DefaultsFixture {
        fn quoted_call(&self, site: Span) -> String {
            let note = self.notes.iter().find(|note| note.span().contains(site));
            let written = note.map_or(site, Form::span);
            format!("{}: `{}`", self.location(written), self.text_at(written))
        }

        fn location(&self, site: Span) -> String {
            let name = self.sources.file(site.source()).name();
            format!("{name}:{}", self.sources.start_position(site))
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
    fn a_second_default_of_one_lint_points_at_the_first() {
        let fixture = DefaultsFixture::read(
            "(unused-binding stop) (needless-type hush) (needless-type note)",
        );
        let Err(problem) = &fixture.read else {
            panic!("the second default is refused");
        };
        let first = problem.first_site().expect("a repeat points at the first");
        let first_start = "(unused-binding stop) (".len();
        assert_eq!(
            first.range(),
            first_start..first_start + "needless-type".len()
        );
    }

    #[test]
    fn an_unknown_lint_is_offered_the_closest_lint() {
        let fixture = DefaultsFixture::read("(needles-type hush)");
        assert_eq!(
            fixture.refusal().1,
            "names `needles-type`, which is no lint; the closest lint is `needless-type`"
        );
    }

    #[test]
    fn an_unknown_level_is_offered_the_closest_level() {
        let fixture = DefaultsFixture::read("(needless-type hsh)");
        assert_eq!(
            fixture.refusal().1,
            "names `hsh`, which is no lint level; the closest level is `hush`"
        );
    }

    #[test]
    fn the_message_of_a_repeat_ends_at_the_first_default() {
        let fixture = DefaultsFixture::read(
            "(unused-binding stop) (needless-type hush) (needless-type note)",
        );
        assert_eq!(
            fixture.message(),
            "lint.knv:1:44: `(needless-type note)` gives `needless-type` a second default \
             level; the first is at lint.knv:1:24"
        );
    }

    #[test]
    fn the_message_of_another_refusal_names_no_first_default() {
        let fixture = DefaultsFixture::read("(tidy-names hush)");
        assert_eq!(
            fixture.message(),
            "lint.knv:1:1: `(tidy-names hush)` names `tidy-names`, which is no lint"
        );
    }

    #[test]
    fn only_a_repeat_points_at_a_first_default() {
        let fixture = DefaultsFixture::read("(tidy-names hush)");
        let Err(problem) = &fixture.read else {
            panic!("an unknown lint is refused");
        };
        assert_eq!(problem.first_site(), None);
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
