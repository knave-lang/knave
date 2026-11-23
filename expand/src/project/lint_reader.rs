use std::rc::Rc;

use super::lint_level::LintLevel;
use super::lint_level_problems::LintLevelProblems;
use super::lint_name_problems::LintNameProblems;
use super::lint_table::LintTable;
use super::vocabulary::ProjectVocabulary;
use crate::diagnostic::{CodeStage, DiagnosticCode, Diagnostics};
use crate::reader::{Form, FormKind};
use crate::source::Span;

pub struct LintReader<'p> {
    vocabulary: &'p ProjectVocabulary,
    problems: &'p mut Diagnostics,
}

impl<'p> LintReader<'p> {
    pub fn new(vocabulary: &'p ProjectVocabulary, problems: &'p mut Diagnostics) -> Self {
        Self {
            vocabulary,
            problems,
        }
    }

    pub fn read(mut self, entries: &[Form], table: &mut LintTable) {
        for entry in entries {
            self.read_entry(entry, table);
        }
    }

    fn read_entry(&mut self, entry: &Form, table: &mut LintTable) {
        let Some((name, parts)) = entry.as_list().and_then(|items| items.split_first()) else {
            return self.without_level(entry);
        };
        let Some((code, written)) = self.code_of(name) else {
            return;
        };
        if parts.is_empty() {
            return self.without_level(entry);
        }
        let Some(level) = self.level_of(parts) else {
            return;
        };
        match table.span_of(&code) {
            Some(first) => {
                self.problems
                    .push(LintNameProblems::listed_twice(written, name.span(), first))
            }
            None => table.add(code, level, name.span()),
        }
    }

    fn code_of<'n>(&mut self, name: &'n Form) -> Option<(DiagnosticCode, &'n str)> {
        let Some(symbol) = name.as_symbol() else {
            self.report_name_not_a_symbol(name);
            return None;
        };
        let code = CodeStage::LINTS.code_named(symbol.name());
        if code.is_none() {
            self.report_unknown_lint(symbol.name(), name.span());
        }
        code.map(|code| (code, symbol.name()))
    }

    fn report_name_not_a_symbol(&mut self, name: &Form) {
        let lints = CodeStage::LINTS.names();
        let written = Self::string_of(name);
        let quoted_code = written.and_then(|text| Some((text, Self::name_of_code(text, &lints)?)));
        let problem = match quoted_code {
            Some((code, lint)) => LintNameProblems::quoted_code(code, name.span(), lint),
            None => LintNameProblems::name_not_a_symbol(
                name.span(),
                written.filter(|text| lints.iter().any(|lint| &**lint == *text)),
            ),
        };
        self.problems.push(problem);
    }

    fn report_unknown_lint(&mut self, text: &str, span: Span) {
        let lints = CodeStage::LINTS.names();
        let problem = match Self::name_of_code(text, &lints) {
            Some(name) => LintNameProblems::code_as_name(text, span, name),
            None => LintNameProblems::unknown_lint(text, span, &lints),
        };
        self.problems.push(problem);
    }

    fn name_of_code<'l>(text: &str, lints: &'l [Rc<str>]) -> Option<&'l str> {
        lints
            .iter()
            .find(|name| {
                CodeStage::LINTS
                    .code_named(name)
                    .is_some_and(|code| code.as_str() == text)
            })
            .map(|name| &**name)
    }

    fn level_of(&mut self, parts: &[Form]) -> Option<LintLevel> {
        let mut found: Option<(LintLevel, Span)> = None;
        for part in parts {
            let Some(level) = self.part_level(part) else {
                continue;
            };
            match found {
                Some((_, first)) => self
                    .problems
                    .push(LintLevelProblems::two_levels(part.span(), first)),
                None => found = Some((level, part.span())),
            }
        }
        found.map(|(level, _)| level)
    }

    fn part_level(&mut self, part: &Form) -> Option<LintLevel> {
        match part.kind() {
            FormKind::Symbol(symbol) => self.level_named(symbol.name(), part.span()),
            FormKind::Number(_) => {
                self.problems
                    .push(LintLevelProblems::threshold(part.span()));
                None
            }
            FormKind::String(_) | FormKind::List(_) => {
                self.report_level_not_a_symbol(part);
                None
            }
        }
    }

    fn level_named(&mut self, word: &str, span: Span) -> Option<LintLevel> {
        let level = self.vocabulary.level_named(word);
        if level.is_none() {
            let levels = self.vocabulary.level_words();
            self.problems
                .push(LintLevelProblems::unknown_level(word, span, &levels));
        }
        level
    }

    fn report_level_not_a_symbol(&mut self, part: &Form) {
        let written =
            Self::string_of(part).filter(|word| self.vocabulary.level_named(word).is_some());
        let levels = self.vocabulary.level_words();
        self.problems.push(LintLevelProblems::level_not_a_symbol(
            part.span(),
            written,
            &levels,
        ));
    }

    fn without_level(&mut self, entry: &Form) {
        let levels = self.vocabulary.level_words();
        self.problems
            .push(LintLevelProblems::without_level(entry.span(), &levels));
    }

    fn string_of(form: &Form) -> Option<&str> {
        match form.kind() {
            FormKind::String(text) => Some(text),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostic::{DiagnosticCode, Diagnostics};
    use crate::project::level_words::LevelWords;
    use crate::project::section_names::SectionNames;
    use crate::project::{LintLevel, Project, ProjectReader, ProjectVocabulary};
    use crate::reader::Reader;
    use crate::source::{SourceMap, Span};

    struct LintsFixture;

    impl LintsFixture {
        fn vocabulary() -> ProjectVocabulary {
            let sections = SectionNames {
                libraries: "libraries".to_string(),
                dependencies: "dependencies".to_string(),
                dev_dependencies: "dev-dependencies".to_string(),
                lints: "lints".to_string(),
                exported_tasks: "exported-tasks".to_string(),
            };
            let levels = LevelWords::new(
                LintLevel::ALL
                    .into_iter()
                    .zip(["allow", "warn", "deny"])
                    .map(|(level, word)| (level, word.to_string()))
                    .collect(),
            );
            ProjectVocabulary::new("defproject".to_string(), sections, levels)
        }

        fn read(entries: &str) -> Result<Project, Diagnostics> {
            let text = format!("(defproject demo (lints {entries}))\n");
            let mut sources = SourceMap::default();
            let source = sources.add("project", &text);
            let forms = Reader::new(sources.file(source))
                .read()
                .expect("the project reads");
            let file = Span::new(source, 0..text.len());
            ProjectReader::new(&Self::vocabulary()).read(&forms, file)
        }

        fn codes(entries: &str) -> Vec<String> {
            let problems = Self::read(entries).err().unwrap_or_default();
            problems
                .in_source_order()
                .iter()
                .filter_map(|problem| problem.code())
                .map(|code| code.as_str().to_string())
                .collect()
        }
    }

    #[test]
    fn each_entry_takes_its_level() {
        let project = LintsFixture::read("(unused-binding allow) (unreachable-arm deny)")
            .expect("the section reads");
        let levels: Vec<(String, LintLevel)> = project
            .lints
            .levels()
            .map(|(code, level)| (code.as_str().to_string(), level))
            .collect();
        assert_eq!(
            levels,
            [
                (
                    DiagnosticCode::UNUSED_BINDING.as_str().to_string(),
                    LintLevel::Allow
                ),
                (
                    DiagnosticCode::UNREACHABLE_ARM.as_str().to_string(),
                    LintLevel::Deny
                ),
            ]
        );
    }

    #[test]
    fn a_warned_entry_takes_the_warn_level() {
        let project = LintsFixture::read("(unused-binding warn)").expect("the section reads");
        let levels: Vec<LintLevel> = project.lints.levels().map(|(_, level)| level).collect();
        assert_eq!(levels, [LintLevel::Warn]);
    }

    #[test]
    fn an_unknown_lint_fails() {
        assert_eq!(
            LintsFixture::codes("(unused-bindings allow)"),
            [DiagnosticCode::UNKNOWN_LINT.as_str()]
        );
    }

    #[test]
    fn an_unknown_level_fails() {
        assert_eq!(
            LintsFixture::codes("(unused-binding hush)"),
            [DiagnosticCode::UNKNOWN_LINT_LEVEL.as_str()]
        );
    }

    #[test]
    fn a_lint_listed_twice_fails() {
        assert_eq!(
            LintsFixture::codes("(unused-binding allow) (unused-binding deny)"),
            [DiagnosticCode::LINT_LISTED_TWICE.as_str()]
        );
    }

    #[test]
    fn an_entry_without_a_level_fails() {
        assert_eq!(
            LintsFixture::codes("(unused-binding)"),
            [DiagnosticCode::LINT_WITHOUT_LEVEL.as_str()]
        );
    }

    #[test]
    fn a_bare_name_fails_as_an_entry_without_a_level() {
        assert_eq!(
            LintsFixture::codes("unused-binding"),
            [DiagnosticCode::LINT_WITHOUT_LEVEL.as_str()]
        );
    }

    #[test]
    fn an_entry_named_by_a_string_fails_as_a_name_that_is_no_symbol() {
        assert_eq!(
            LintsFixture::codes("(\"unused-binding\" allow)"),
            [DiagnosticCode::LINT_NAME_NOT_A_SYMBOL.as_str()]
        );
    }

    #[test]
    fn a_level_written_as_a_string_fails_as_a_level_that_is_no_symbol() {
        assert_eq!(
            LintsFixture::codes("(unused-binding \"allow\")"),
            [DiagnosticCode::LINT_LEVEL_NOT_A_SYMBOL.as_str()]
        );
    }

    #[test]
    fn a_level_written_as_a_list_fails_as_a_level_that_is_no_symbol() {
        assert_eq!(
            LintsFixture::codes("(unused-binding (allow))"),
            [DiagnosticCode::LINT_LEVEL_NOT_A_SYMBOL.as_str()]
        );
    }

    #[test]
    fn a_lint_written_as_its_code_fails_as_an_unknown_lint() {
        assert_eq!(
            LintsFixture::codes("(L0801 allow)"),
            [DiagnosticCode::UNKNOWN_LINT.as_str()]
        );
    }

    #[test]
    fn a_threshold_fails() {
        assert_eq!(
            LintsFixture::codes("(unused-binding 3)"),
            [DiagnosticCode::LINT_THRESHOLD.as_str()]
        );
    }

    #[test]
    fn two_levels_in_one_entry_fail() {
        assert_eq!(
            LintsFixture::codes("(unused-binding allow deny)"),
            [DiagnosticCode::DUPLICATE_PROJECT_ENTRY.as_str()]
        );
    }
}
