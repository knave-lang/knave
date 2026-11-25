use std::env;
use std::process::{Command, Output};

use super::fixture_glob::FixtureGlob;
use crate::heavy_run::RunEnvironment;

pub struct HeavyRunProbe {
    command: Command,
}

impl HeavyRunProbe {
    const SNAPSHOT_UPDATE: &str = "INSTA_UPDATE";
    const EXACT: &str = "--exact";

    pub fn of_test(test: &str) -> Self {
        let executable = env::current_exe().expect("the test binary has a path");
        let mut command = Command::new(executable);
        RunEnvironment::clear_from(&mut command)
            .args([test, Self::EXACT])
            .env_remove(FixtureGlob::VARIABLE)
            .env_remove(Self::SNAPSHOT_UPDATE);
        Self { command }
    }

    pub fn on_owner_override(mut self) -> Self {
        self.command.env(RunEnvironment::OWNER_OVERRIDE, "1");
        self
    }

    pub fn narrowed_by(mut self, glob: &str) -> Self {
        self.command.env(FixtureGlob::VARIABLE, glob);
        self
    }

    pub fn output(mut self) -> Output {
        self.command.output().expect("the test binary starts")
    }
}
