use std::sync::Once;

use super::admission::Admission;
use super::narrowing::Narrowing;
use super::run_environment::RunEnvironment;
use super::uncaptured::Uncaptured;

pub struct HeavyTestRun<'a> {
    environment: RunEnvironment,
    narrowing: &'a dyn Narrowing,
}

impl<'a> HeavyTestRun<'a> {
    pub const REFUSAL: &'static str = "error: this is a heavy test run, which runs on the `Heavy tests` workflow and not locally. Dispatch it as the Heavy tests section of docs/pull-requests.md says.";

    pub fn of(narrowing: &'a dyn Narrowing) -> Self {
        Self::within(RunEnvironment::read(), narrowing)
    }

    pub fn within(environment: RunEnvironment, narrowing: &'a dyn Narrowing) -> Self {
        Self {
            environment,
            narrowing,
        }
    }

    pub fn override_notice() -> String {
        format!(
            "warning: {} is set, so this heavy test run runs locally on the project owner's override.",
            RunEnvironment::OWNER_OVERRIDE
        )
    }

    pub fn enforce(&self) {
        match self.admission() {
            Admission::Allowed => {}
            Admission::OnOwnerOverride => Self::announce_override(),
            Admission::Refused => Self::refuse(),
        }
    }

    pub fn admission(&self) -> Admission {
        if self.environment.on_github_actions || self.narrowing.narrows() {
            Admission::Allowed
        } else if self.environment.owner_override {
            Admission::OnOwnerOverride
        } else {
            Admission::Refused
        }
    }

    fn announce_override() {
        static ANNOUNCED: Once = Once::new();
        ANNOUNCED.call_once(|| Uncaptured::write(&Self::override_notice()));
    }

    fn refuse() -> ! {
        static ANNOUNCED: Once = Once::new();
        ANNOUNCED.call_once(|| Uncaptured::write(Self::REFUSAL));
        panic!("{}", Self::REFUSAL)
    }
}
