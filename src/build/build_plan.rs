use super::profile::BuildProfile;

pub struct BuildPlan {
    profile: BuildProfile,
}

impl BuildPlan {
    pub const EDITION: &str = "2024";
    const CAP_LINTS: [&str; 2] = ["--cap-lints", "allow"];
    const CARGO_RELEASE: [&str; 1] = ["--release"];
    const MANIFEST_PROFILE: &str = "[profile.dev]\ndebug = false\n";

    pub fn new(profile: BuildProfile) -> Self {
        Self { profile }
    }

    pub fn lint_flags(&self) -> &'static [&'static str] {
        &Self::CAP_LINTS
    }

    pub fn rustc_flags(&self) -> &'static [&'static str] {
        self.profile.rustc_arguments()
    }

    pub fn cargo_flags(&self) -> &'static [&'static str] {
        match self.profile {
            BuildProfile::Debug => &[],
            BuildProfile::Release => &Self::CARGO_RELEASE,
        }
    }

    pub fn manifest_profile(&self) -> &'static str {
        Self::MANIFEST_PROFILE
    }
}
