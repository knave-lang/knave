use std::rc::Rc;

use super::section::Section;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectVocabulary {
    head: String,
    sections: SectionNames,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct SectionNames {
    pub(super) libraries: String,
    pub(super) dependencies: String,
    pub(super) dev_dependencies: String,
}

impl ProjectVocabulary {
    pub(super) fn new(head: String, sections: SectionNames) -> Self {
        Self { head, sections }
    }

    pub fn head(&self) -> &str {
        &self.head
    }

    pub fn section_name(&self, section: Section) -> &str {
        match section {
            Section::Libraries => &self.sections.libraries,
            Section::Dependencies => &self.sections.dependencies,
            Section::DevDependencies => &self.sections.dev_dependencies,
        }
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
    use super::{ProjectVocabulary, SectionNames};
    use crate::project::Section;

    struct VocabularyFixture;

    impl VocabularyFixture {
        fn vocabulary() -> ProjectVocabulary {
            let sections = SectionNames {
                libraries: "books".to_string(),
                dependencies: "crates".to_string(),
                dev_dependencies: "test-crates".to_string(),
            };
            ProjectVocabulary::new("opening".to_string(), sections)
        }
    }

    #[test]
    fn each_section_has_its_own_name() {
        let vocabulary = VocabularyFixture::vocabulary();
        let names: Vec<&str> = Section::ALL
            .iter()
            .map(|section| vocabulary.section_name(*section))
            .collect();
        assert_eq!(names, ["books", "crates", "test-crates"]);
    }

    #[test]
    fn a_section_name_finds_its_section() {
        let found = VocabularyFixture::vocabulary().section_named("crates");
        assert_eq!(found, Some(Section::Dependencies));
    }

    #[test]
    fn the_head_is_no_section() {
        assert_eq!(
            VocabularyFixture::vocabulary().section_named("opening"),
            None
        );
    }
}
