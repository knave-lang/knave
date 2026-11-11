use clap::Args;

use crate::build::BuildProfile;

#[derive(Args, Default)]
pub struct ReleaseFlag {
    #[arg(long)]
    release: bool,
}

impl ReleaseFlag {
    pub fn profile(&self) -> BuildProfile {
        if self.release {
            BuildProfile::Release
        } else {
            BuildProfile::Debug
        }
    }
}
