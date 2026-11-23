use std::rc::Rc;

use super::dependency_section::DependencySection;
use super::level_words::LevelWords;
use super::lint_level::LintLevel;
use super::section::Section;
use super::section_names::SectionNames;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectVocabulary {
    head: String,
    sections: SectionNames,
    levels: LevelWords,
}

impl ProjectVocabulary {
    pub(super) fn new(head: String, sections: SectionNames, levels: LevelWords) -> Self {
        Self {
            head,
            sections,
            levels,
        }
    }

    pub fn head(&self) -> &str {
        &self.head
    }

    pub fn section_name(&self, section: Section) -> &str {
        match section {
            Section::Dependency(DependencySection::Libraries) => &self.sections.libraries,
            Section::Dependency(DependencySection::Dependencies) => &self.sections.dependencies,
            Section::Dependency(DependencySection::DevDependencies) => {
                &self.sections.dev_dependencies
            }
            Section::Lints => &self.sections.lints,
            Section::ExportedTasks => &self.sections.exported_tasks,
        }
    }

    pub(super) fn level_named(&self, word: &str) -> Option<LintLevel> {
        self.levels.level_named(word)
    }

    pub(super) fn level_words(&self) -> Vec<Rc<str>> {
        self.levels.words()
    }

    pub(super) fn section_named(&self, name: &str) -> Option<Section> {
        Section::ALL
            .into_iter()
            .find(|section| self.section_name(*section) == name)
    }

    pub(super) fn section_names(&self) -> Vec<Rc<str>> {
        Section::ALL
            .iter()
            .map(|section| Rc::from(self.section_name(*section)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectVocabulary;
    use crate::project::LintLevel;
    use crate::project::level_words::LevelWords;
    use crate::project::section_names::SectionNames;
    use crate::project::{DependencySection, Section};

    struct VocabularyFixture;

    impl VocabularyFixture {
        fn vocabulary() -> ProjectVocabulary {
            let sections = SectionNames {
                libraries: "books".to_string(),
                dependencies: "crates".to_string(),
                dev_dependencies: "test-crates".to_string(),
                lints: "rules".to_string(),
                exported_tasks: "offers".to_string(),
            };
            let levels = LevelWords::new(
                LintLevel::ALL
                    .into_iter()
                    .zip(["hush", "note", "stop"])
                    .map(|(level, word)| (level, word.to_string()))
                    .collect(),
            );
            ProjectVocabulary::new("opening".to_string(), sections, levels)
        }
    }

    #[test]
    fn each_section_has_its_own_name() {
        let vocabulary = VocabularyFixture::vocabulary();
        let names: Vec<&str> = Section::ALL
            .iter()
            .map(|section| vocabulary.section_name(*section))
            .collect();
        assert_eq!(names, ["books", "crates", "test-crates", "rules", "offers"]);
    }

    #[test]
    fn a_section_name_finds_its_section() {
        let found = VocabularyFixture::vocabulary().section_named("crates");
        assert_eq!(
            found,
            Some(Section::Dependency(DependencySection::Dependencies))
        );
    }

    #[test]
    fn the_head_is_no_section() {
        assert_eq!(
            VocabularyFixture::vocabulary().section_named("opening"),
            None
        );
    }
}
