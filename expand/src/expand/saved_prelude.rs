use super::prelude_state::{PreludeState, UnreadablePrelude};
use super::project_file_module::ProjectFileModule;
use crate::project::ProjectVocabulary;

pub struct SavedPrelude;

impl SavedPrelude {
    pub fn build() -> Vec<u8> {
        Self::write(
            &PreludeState::build(),
            &ProjectFileModule::expanded().vocabulary(),
        )
    }

    pub fn state(saved: &'static [u8]) -> Result<PreludeState, UnreadablePrelude> {
        let vocabulary = Self::vocabulary(saved)?;
        let start = bincode::serialized_size(vocabulary).map_err(UnreadablePrelude)?;
        let start = usize::try_from(start).expect("the saved prelude fits in memory");
        PreludeState::load(&saved[start..])
    }

    pub fn project_vocabulary(saved: &'static [u8]) -> ProjectVocabulary {
        let vocabulary =
            Self::vocabulary(saved).expect("the saved prelude opens with its project vocabulary");
        bincode::deserialize(vocabulary).expect("the saved project vocabulary decodes")
    }

    fn write(state: &PreludeState, vocabulary: &ProjectVocabulary) -> Vec<u8> {
        let encoded = bincode::serialize(vocabulary).expect("the project vocabulary serializes");
        let mut saved =
            bincode::serialize(&encoded).expect("the project vocabulary's bytes serialize");
        saved.extend(state.save());
        saved
    }

    fn vocabulary(saved: &'static [u8]) -> Result<&'static [u8], UnreadablePrelude> {
        bincode::deserialize(saved).map_err(UnreadablePrelude)
    }
}

#[cfg(test)]
mod tests {
    use super::SavedPrelude;
    use crate::expand::PreludeState;
    use crate::expand::project_file_module::ProjectFileModule;
    use crate::project::Section;

    #[test]
    fn the_saved_prelude_reads_back_its_vocabulary_and_its_state() {
        let vocabulary = ProjectFileModule::expanded().vocabulary();
        let saved = SavedPrelude::write(&PreludeState::build(), &vocabulary).leak();
        assert_eq!(SavedPrelude::project_vocabulary(saved), vocabulary);
        assert!(SavedPrelude::state(saved).is_ok());
    }

    #[test]
    fn a_garbled_saved_prelude_is_unreadable() {
        let garbled: &'static [u8] = vec![0xff; 4].leak();
        assert!(SavedPrelude::state(garbled).is_err());
    }

    #[test]
    fn the_prelude_names_its_project_head_and_each_section() {
        let vocabulary = ProjectFileModule::expanded().vocabulary();
        assert_eq!(vocabulary.head(), "defproject");
        let sections = [
            Section::Libraries,
            Section::Dependencies,
            Section::DevDependencies,
        ];
        let names = sections.map(|section| vocabulary.section_name(section));
        assert_eq!(names, ["libraries", "dependencies", "dev-dependencies"]);
    }
}
