use std::env;
use std::io::{self, Write};
use std::process;
use std::sync::Once;

pub trait Narrowing {
    fn narrows(&self) -> bool;
}

pub struct LocalEnvironment {
    pub in_ci: bool,
    pub owner_override: bool,
}

#[derive(Debug, PartialEq)]
pub enum LocalRun {
    Allowed,
    OnOwnerOverride,
    Refused,
}

pub struct HeavyTestRun<'a> {
    environment: LocalEnvironment,
    narrowing: &'a dyn Narrowing,
}

impl LocalEnvironment {
    pub const OWNER_OVERRIDE: &'static str = "KNAVE_OWNER_LOCAL_OVERRIDE";
    const CI: &'static str = "CI";

    pub fn read() -> Self {
        Self {
            in_ci: env::var_os(Self::CI).is_some(),
            owner_override: env::var_os(Self::OWNER_OVERRIDE).is_some(),
        }
    }
}

impl<'a> HeavyTestRun<'a> {
    pub const REFUSAL: &'static str = "error: this is a heavy test run, which runs on the `Heavy tests` workflow and not locally. Dispatch it as the Heavy tests section of docs/pull-requests.md says.";
    pub const OVERRIDE_NOTICE: &'static str = "warning: KNAVE_OWNER_LOCAL_OVERRIDE is set, so this heavy test run runs locally on the project owner's override.";
    const REFUSED_EXIT_CODE: i32 = 1;

    pub fn of(narrowing: &'a dyn Narrowing) -> Self {
        Self::within(LocalEnvironment::read(), narrowing)
    }

    pub fn within(environment: LocalEnvironment, narrowing: &'a dyn Narrowing) -> Self {
        Self {
            environment,
            narrowing,
        }
    }

    pub fn enforce(&self) {
        match self.local_run() {
            LocalRun::Allowed => {}
            LocalRun::OnOwnerOverride => Self::announce_override(),
            LocalRun::Refused => Self::refuse(),
        }
    }

    pub fn local_run(&self) -> LocalRun {
        if self.environment.in_ci || self.narrowing.narrows() {
            LocalRun::Allowed
        } else if self.environment.owner_override {
            LocalRun::OnOwnerOverride
        } else {
            LocalRun::Refused
        }
    }

    fn announce_override() {
        static ANNOUNCED: Once = Once::new();
        ANNOUNCED.call_once(|| Self::write_past_the_capture(Self::OVERRIDE_NOTICE));
    }

    fn refuse() -> ! {
        Self::write_past_the_capture(Self::REFUSAL);
        process::exit(Self::REFUSED_EXIT_CODE)
    }

    fn write_past_the_capture(line: &str) {
        let _ = writeln!(io::stderr(), "{line}");
    }
}
