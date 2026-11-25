use std::fs;

use super::stage_run::StageRun;

pub struct StageTestSource {
    text: String,
}

impl StageTestSource {
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/stages.rs");
    const FUNCTION_PREFIX: &str = "fn ";
    const PARAMETERS_START: char = '(';
    const GUARD: &str = "    HeavyTestRun::of(&StageRun::from_process()).enforce();";

    pub fn read() -> Self {
        Self {
            text: fs::read_to_string(Self::PATH).expect("the stage tests read"),
        }
    }

    pub fn heavy_test_names(&self) -> Vec<&str> {
        self.test_names_where(StageRun::is_heavy_name)
    }

    pub fn guarded_test_names(&self) -> Vec<&str> {
        self.test_names_where(StageRun::is_guarded_name)
    }

    fn test_names_where(&self, keeps: fn(&str) -> bool) -> Vec<&str> {
        self.text
            .lines()
            .filter_map(|line| line.strip_prefix(Self::FUNCTION_PREFIX))
            .filter_map(|line| line.split(Self::PARAMETERS_START).next())
            .filter(|name| keeps(name))
            .collect()
    }

    pub fn opens_with_guard(&self, test: &str) -> bool {
        let opening = format!("fn {test}() {{\n{}\n", Self::GUARD);
        self.text.contains(&opening)
    }
}
