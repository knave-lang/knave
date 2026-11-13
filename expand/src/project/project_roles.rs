use rustc_hash::{FxHashMap, FxHashSet};

use super::vocabulary::{ProjectVocabulary, SectionNames};
use crate::reader::Form;

pub(crate) struct ProjectRoles<'m> {
    macros: &'m FxHashSet<String>,
    name_of_role: FxHashMap<String, String>,
    names_with_a_role: FxHashSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RoleProblem {
    Missing(&'static str),
    Repeated(String),
    Unknown(String),
    TwoRoles(String),
    NoMacro(String),
    Malformed,
}

impl<'m> ProjectRoles<'m> {
    pub(crate) const NOTE: &'static str = "project-roles";

    pub(crate) fn new(macros: &'m FxHashSet<String>) -> Self {
        Self {
            macros,
            name_of_role: FxHashMap::default(),
            names_with_a_role: FxHashSet::default(),
        }
    }

    pub(crate) fn read(mut self, notes: &[Form]) -> Result<ProjectVocabulary, RoleProblem> {
        self.add_all(notes)?;
        let vocabulary = ProjectVocabulary::new(self.taken("project-head")?, self.sections()?);
        self.unknown_left_over()?;
        Ok(vocabulary)
    }

    fn add_all(&mut self, notes: &[Form]) -> Result<(), RoleProblem> {
        for note in notes {
            let (name, role) = Self::parts(note)?;
            self.add(name, role)?;
        }
        Ok(())
    }

    fn sections(&mut self) -> Result<SectionNames, RoleProblem> {
        Ok(SectionNames {
            libraries: self.taken("library-section")?,
            dependencies: self.taken("crate-section")?,
            dev_dependencies: self.taken("test-crate-section")?,
        })
    }

    fn add(&mut self, name: String, role: String) -> Result<(), RoleProblem> {
        self.check_name(&name)?;
        if self.name_of_role.contains_key(&role) {
            return Err(RoleProblem::Repeated(role));
        }
        self.names_with_a_role.insert(name.clone());
        self.name_of_role.insert(role, name);
        Ok(())
    }

    fn check_name(&self, name: &str) -> Result<(), RoleProblem> {
        if !self.macros.contains(name) {
            return Err(RoleProblem::NoMacro(name.to_string()));
        }
        if self.names_with_a_role.contains(name) {
            return Err(RoleProblem::TwoRoles(name.to_string()));
        }
        Ok(())
    }

    fn taken(&mut self, role: &'static str) -> Result<String, RoleProblem> {
        self.name_of_role
            .remove(role)
            .ok_or(RoleProblem::Missing(role))
    }

    fn unknown_left_over(&self) -> Result<(), RoleProblem> {
        match self.name_of_role.keys().next() {
            Some(role) => Err(RoleProblem::Unknown(role.clone())),
            None => Ok(()),
        }
    }

    fn parts(note: &Form) -> Result<(String, String), RoleProblem> {
        let Some([name, role]) = note.as_list() else {
            return Err(RoleProblem::Malformed);
        };
        let symbol = |form: &Form| form.as_symbol().map(|symbol| symbol.name().to_string());
        symbol(name).zip(symbol(role)).ok_or(RoleProblem::Malformed)
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::{ProjectRoles, RoleProblem};
    use crate::project::ProjectVocabulary;
    use crate::reader::{Form, Reader};
    use crate::source::SourceMap;

    struct RolesFixture;

    impl RolesFixture {
        const MACROS: [&str; 5] = ["opening", "books", "crates", "test-crates", "spare"];
        const ROLES: &str = "(opening project-head)\n\
             (books library-section)\n\
             (crates crate-section)\n\
             (test-crates test-crate-section)\n";

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
            ProjectRoles::new(&macros).read(&Self::notes(roles))
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
