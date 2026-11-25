use std::env;

use crate::heavy_run::Narrowing;

pub struct FixtureGlob {
    filter: Option<String>,
}

impl FixtureGlob {
    const VARIABLE: &str = "INSTA_GLOB_FILTER";
    const SEPARATOR: char = ';';
    const PATH_SEPARATOR: char = '/';
    const WILDCARDS: [char; 2] = ['*', '?'];

    pub fn from_environment() -> Self {
        Self::of(env::var(Self::VARIABLE).ok())
    }

    pub fn of(filter: Option<String>) -> Self {
        Self { filter }
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
        !file.contains(Self::WILDCARDS)
    }
}

impl Narrowing for FixtureGlob {
    fn narrows(&self) -> bool {
        let globs = self.globs();
        !globs.is_empty() && globs.iter().all(|glob| Self::names_a_file(glob))
    }
}
