use std::env;
use std::path::{Path, PathBuf};

pub struct LeftOutFixtures {
    names: &'static [&'static str],
}

impl LeftOutFixtures {
    const LEAVE_OUT_SLOW: &str = "KNAVE_LEAVE_OUT_SLOW_FIXTURES";
    const SLOW_UNDER_COVERAGE: &[&str] = &["check/macro_vec_of_twenty_thousand_elements.knv"];

    pub fn from_environment() -> Self {
        if env::var_os(Self::LEAVE_OUT_SLOW).is_some() {
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

    pub fn none() -> Self {
        Self { names: &[] }
    }

    pub fn kept(&self, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        paths
            .into_iter()
            .filter(|path| !self.leaves_out(path))
            .collect()
    }

    fn leaves_out(&self, path: &Path) -> bool {
        self.names.iter().any(|name| path.ends_with(name))
    }
}
