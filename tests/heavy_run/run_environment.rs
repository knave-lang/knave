use std::env;
use std::process::Command;

pub struct RunEnvironment {
    pub on_github_actions: bool,
    pub owner_override: bool,
}

impl RunEnvironment {
    pub const OWNER_OVERRIDE: &'static str = "KNAVE_OWNER_LOCAL_OVERRIDE";
    const GITHUB_ACTIONS: &'static str = "GITHUB_ACTIONS";
    const GITHUB_ACTIONS_SET: &'static str = "true";
    const VARIABLES: [&'static str; 2] = [Self::GITHUB_ACTIONS, Self::OWNER_OVERRIDE];

    pub fn read() -> Self {
        Self {
            on_github_actions: env::var(Self::GITHUB_ACTIONS).as_deref()
                == Ok(Self::GITHUB_ACTIONS_SET),
            owner_override: env::var_os(Self::OWNER_OVERRIDE).is_some(),
        }
    }

    pub fn cleared_test_binary() -> Command {
        let executable = env::current_exe().expect("the test binary has a path");
        let mut command = Command::new(executable);
        for variable in Self::VARIABLES {
            command.env_remove(variable);
        }
        command
    }
}
