use std::env;
use std::process::{Command, Output};

use crate::heavy_run::LocalEnvironment;

pub struct HeavyRunProbe {
    command: Command,
}

impl HeavyRunProbe {
    const CI: &str = "CI";
    const GLOB_FILTER: &str = "INSTA_GLOB_FILTER";
    const SNAPSHOT_UPDATE: &str = "INSTA_UPDATE";
    const EXACT: &str = "--exact";

    pub fn of_test(test: &str) -> Self {
        let executable = env::current_exe().expect("the test binary has a path");
        let mut command = Command::new(executable);
        command
            .args([test, Self::EXACT])
            .env_remove(Self::CI)
            .env_remove(LocalEnvironment::OWNER_OVERRIDE)
            .env_remove(Self::GLOB_FILTER)
            .env_remove(Self::SNAPSHOT_UPDATE);
        Self { command }
    }

    pub fn on_owner_override(mut self) -> Self {
        self.command.env(LocalEnvironment::OWNER_OVERRIDE, "1");
        self
    }

    pub fn narrowed_by(mut self, glob: &str) -> Self {
        self.command.env(Self::GLOB_FILTER, glob);
        self
    }

    pub fn output(mut self) -> Output {
        self.command.output().expect("the test binary starts")
    }
}
