use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct LeftOutFixtures {
    names: &'static [&'static str],
}

impl LeftOutFixtures {
    const VARIABLE: &str = "KNAVE_LEAVE_OUT_SLOW_FIXTURES";
    const LEAVE_OUT: &str = "1";
    const SLOW_UNDER_COVERAGE: &[&str] = &["check/macro_vec_of_twenty_thousand_elements.knv"];
    const REASON: &str = "it runs too slowly under prepush's coverage build; CI still runs it";

    pub fn leave_out_slow_fixtures(command: &mut Command) -> &mut Command {
        command.env(Self::VARIABLE, Self::LEAVE_OUT)
    }

    pub fn from_environment() -> Self {
        Self::from_variable(env::var_os(Self::VARIABLE).as_deref())
    }

    pub fn from_variable(value: Option<&OsStr>) -> Self {
        if value == Some(OsStr::new(Self::LEAVE_OUT)) {
            Self::slow()
        } else {
            Self::none()
        }
    }

    pub fn slow() -> Self {
        Self {
            names: Self::SLOW_UNDER_COVERAGE,
        }
    }

    fn none() -> Self {
        Self { names: &[] }
    }

    pub fn kept(&self, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        paths
            .into_iter()
            .filter(|path| self.left_out_name(path).is_none())
            .collect()
    }

    pub fn notes(&self) -> Vec<String> {
        self.names.iter().map(|name| Self::note(name)).collect()
    }

    pub fn notes_for(&self, paths: &[PathBuf]) -> Vec<String> {
        paths
            .iter()
            .filter_map(|path| self.left_out_name(path))
            .map(Self::note)
            .collect()
    }

    fn note(name: &str) -> String {
        format!("left out the fixture {name}: {}", Self::REASON)
    }

    fn left_out_name(&self, path: &Path) -> Option<&'static str> {
        self.names.iter().copied().find(|name| path.ends_with(name))
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::PathBuf;
    use std::process::Command;

    use super::LeftOutFixtures;

    struct LeftOutCase;

    impl LeftOutCase {
        fn slow_path() -> PathBuf {
            PathBuf::from("tests/fixtures").join(LeftOutFixtures::SLOW_UNDER_COVERAGE[0])
        }

        fn quick_path() -> PathBuf {
            PathBuf::from("tests/fixtures/check/quick.knv")
        }

        fn kept_by(fixtures: &LeftOutFixtures) -> Vec<PathBuf> {
            fixtures.kept(vec![Self::slow_path(), Self::quick_path()])
        }
    }

    #[test]
    fn a_command_asked_to_leave_out_slow_fixtures_makes_its_tests_leave_them_out() {
        let mut command = Command::new("cargo");
        LeftOutFixtures::leave_out_slow_fixtures(&mut command);
        let (_, value) = command.get_envs().next().expect("one variable is set");
        let fixtures = LeftOutFixtures::from_variable(value);
        assert_eq!(LeftOutCase::kept_by(&fixtures), [LeftOutCase::quick_path()]);
    }

    #[test]
    fn an_unset_variable_keeps_every_fixture() {
        let fixtures = LeftOutFixtures::from_variable(None);
        assert_eq!(LeftOutCase::kept_by(&fixtures).len(), 2);
    }

    #[test]
    fn a_variable_set_to_another_value_keeps_every_fixture() {
        for value in ["0", ""] {
            let fixtures = LeftOutFixtures::from_variable(Some(OsStr::new(value)));
            assert_eq!(LeftOutCase::kept_by(&fixtures).len(), 2, "{value:?}");
        }
    }

    #[test]
    fn a_note_names_each_left_out_fixture_among_the_paths_and_why() {
        let notes = LeftOutFixtures::slow()
            .notes_for(&[LeftOutCase::slow_path(), LeftOutCase::quick_path()]);
        assert_eq!(
            notes,
            [format!(
                "left out the fixture {}: {}",
                LeftOutFixtures::SLOW_UNDER_COVERAGE[0],
                LeftOutFixtures::REASON
            )]
        );
    }

    #[test]
    fn no_note_names_a_fixture_absent_from_the_paths() {
        let notes = LeftOutFixtures::slow().notes_for(&[LeftOutCase::quick_path()]);
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn the_notes_of_every_slow_fixture_match_the_notes_for_their_paths() {
        let fixtures = LeftOutFixtures::slow();
        assert_eq!(
            fixtures.notes(),
            fixtures.notes_for(&[LeftOutCase::slow_path()])
        );
    }
}
