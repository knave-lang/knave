use rustc_hash::FxHashSet;

use super::level_words::LevelWords;
use super::role_book::RoleBook;
use super::role_note::RoleNote;
use super::role_problem::RoleProblem;
use super::section_names::SectionNames;
use super::vocabulary::ProjectVocabulary;
use crate::reader::Form;

pub(crate) struct ProjectRoles<'m> {
    macros: &'m FxHashSet<String>,
}

impl<'m> ProjectRoles<'m> {
    pub(crate) const NOTE: &'static str = "project-roles";

    pub(crate) fn new(macros: &'m FxHashSet<String>) -> Self {
        Self { macros }
    }

    pub(crate) fn read(
        self,
        notes: &[Form],
        levels: LevelWords,
    ) -> Result<ProjectVocabulary, RoleProblem> {
        let mut book = RoleBook::of(notes, |note| self.admits(note))?;
        let head = book.taken("project-head")?;
        let sections = Self::sections(&mut book)?;
        book.finish()?;
        Ok(ProjectVocabulary::new(head, sections, levels))
    }

    fn sections(book: &mut RoleBook) -> Result<SectionNames, RoleProblem> {
        Ok(SectionNames {
            libraries: book.taken("library-section")?,
            dependencies: book.taken("crate-section")?,
            dev_dependencies: book.taken("test-crate-section")?,
            lints: book.taken("lint-section")?,
            exported_tasks: book.taken("task-export-section")?,
        })
    }

    fn admits(&self, note: &RoleNote) -> Result<(), RoleProblem> {
        if self.macros.contains(&note.name) {
            return Ok(());
        }
        Err(RoleProblem::NoMacro(note.name.clone()))
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::ProjectRoles;
    use crate::project::LevelKeys;
    use crate::project::role_problem::RoleProblem;
    use crate::project::{ProjectVocabulary, Section};
    use crate::reader::{Form, Reader};
    use crate::source::SourceMap;

    struct RolesFixture;

    impl RolesFixture {
        const MACROS: [&str; 7] = [
            "opening",
            "books",
            "crates",
            "test-crates",
            "rules",
            "offers",
            "spare",
        ];
        const ROLES: &str = "(opening project-head)\n\
             (books library-section)\n\
             (crates crate-section)\n\
             (test-crates test-crate-section)\n\
             (rules lint-section)\n\
             (offers task-export-section)\n";
        const LEVELS: &str = "(hush silent-level) (note warning-level) (stop error-level)";

        fn notes(text: &str) -> Vec<Form> {
            let mut sources = SourceMap::default();
            let source = sources.add("notes", text);
            Reader::new(sources.file(source))
                .read()
                .expect("the notes read")
        }

        fn read(roles: &str) -> Result<ProjectVocabulary, RoleProblem> {
            let macros: FxHashSet<String> =
                Self::MACROS.iter().map(|name| name.to_string()).collect();
            let levels =
                LevelKeys::read(&Self::notes(Self::LEVELS)).expect("every key is stated once");
            ProjectRoles::new(&macros).read(&Self::notes(roles), levels)
        }

        fn with(extra: &str) -> Result<ProjectVocabulary, RoleProblem> {
            Self::read(&format!("{}{extra}", Self::ROLES))
        }
    }

    #[test]
    fn each_role_names_its_form() {
        let vocabulary =
            RolesFixture::read(RolesFixture::ROLES).expect("every role is stated once");
        assert_eq!(vocabulary.head(), "opening");
        assert_eq!(vocabulary.section_named("crates").map(|_| ()), Some(()));
        assert_eq!(vocabulary.section_named("rules"), Some(Section::Lints));
    }

    #[test]
    fn a_missing_role_fails() {
        let roles = RolesFixture::ROLES.replace("(books library-section)\n", "");
        assert_eq!(
            RolesFixture::read(&roles),
            Err(RoleProblem::Missing("library-section"))
        );
    }

    #[test]
    fn a_repeated_role_fails() {
        let failure = RolesFixture::with("(spare crate-section)\n");
        assert_eq!(
            failure,
            Err(RoleProblem::Repeated("crate-section".to_string()))
        );
    }

    #[test]
    fn an_unknown_role_fails() {
        let failure = RolesFixture::with("(spare shelf-section)\n");
        assert_eq!(
            failure,
            Err(RoleProblem::Unknown("shelf-section".to_string()))
        );
    }

    #[test]
    fn a_name_with_two_roles_fails() {
        let roles = RolesFixture::ROLES.replace("(crates crate-section)", "(books crate-section)");
        assert_eq!(
            RolesFixture::read(&roles),
            Err(RoleProblem::TwoRoles("books".to_string()))
        );
    }

    #[test]
    fn a_name_the_module_does_not_define_as_a_macro_fails() {
        let failure = RolesFixture::with("(helper crate-section)\n");
        assert_eq!(failure, Err(RoleProblem::NoMacro("helper".to_string())));
    }

    #[test]
    fn a_note_that_is_no_name_and_role_fails() {
        assert_eq!(RolesFixture::with("(spare)\n"), Err(RoleProblem::Malformed));
    }
}
