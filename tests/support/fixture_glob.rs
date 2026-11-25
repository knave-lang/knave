use std::env;
use std::path::PathBuf;
use std::thread;

use globset::Glob;

use crate::heavy_run::Narrowing;
use crate::heavy_run::uncaptured::Uncaptured;

pub struct FixtureGlob {
    filter: Option<String>,
}

impl FixtureGlob {
    pub const VARIABLE: &str = "INSTA_GLOB_FILTER";
    const SEPARATOR: char = ';';
    const PATH_SEPARATOR: char = '/';
    const METACHARACTERS: [char; 4] = ['*', '?', '[', '{'];
    const UNNAMED_TEST: &str = "this test";

    pub fn from_environment() -> Self {
        Self::of(env::var(Self::VARIABLE).ok())
    }

    pub fn of(filter: Option<String>) -> Self {
        Self { filter }
    }

    pub fn report_kept(&self, kept: &[PathBuf]) {
        let test = thread::current()
            .name()
            .unwrap_or(Self::UNNAMED_TEST)
            .to_string();
        if let Some(line) = self.kept_line(kept.len(), &test) {
            Uncaptured::write(&line);
        }
        assert!(
            self.filter.is_none() || !kept.is_empty(),
            "{} keeps no fixture of {test}",
            Self::VARIABLE
        );
    }

    pub fn kept_line(&self, kept: usize, test: &str) -> Option<String> {
        self.filter
            .as_ref()
            .map(|_| format!("{} kept {kept} fixtures in {test}", Self::VARIABLE))
    }

    fn globs(&self) -> Vec<&str> {
        self.filter
            .iter()
            .flat_map(|filter| filter.split(Self::SEPARATOR))
            .filter(|glob| !glob.is_empty())
            .collect()
    }

    fn names_a_file(glob: &str) -> bool {
        let file = glob.rsplit(Self::PATH_SEPARATOR).next().unwrap_or(glob);
        Glob::new(glob).is_ok() && !file.contains(Self::METACHARACTERS)
    }
}

impl Narrowing for FixtureGlob {
    fn narrows(&self) -> bool {
        let globs = self.globs();
        !globs.is_empty() && globs.iter().all(|glob| Self::names_a_file(glob))
    }
}
