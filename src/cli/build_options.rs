use std::path::PathBuf;

use crate::build::BuildProfile;

pub struct BuildOptions {
    pub profile: BuildProfile,
    pub output: Option<PathBuf>,
}
