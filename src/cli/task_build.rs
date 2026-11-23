use std::path::{Path, PathBuf};

use super::build_route::BuildRoute;
use super::build_routes::BuildRoutes;
use super::crate_name::CrateName;
use super::project_crates::ProjectCrates;
use super::rustc_route::RustcRoute;
use super::task_name::TaskName;

pub struct TaskBuild {
    bin_stem: String,
    directory: PathBuf,
    binary: PathBuf,
    crate_name: CrateName,
}

impl TaskBuild {
    pub fn new(bin_stem: String, directory: PathBuf, name: &TaskName) -> Self {
        Self {
            bin_stem,
            binary: directory.join(name.as_str()),
            crate_name: CrateName::of(Path::new(name.as_str())),
            directory,
        }
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    pub fn route<'a>(&'a self, crates: &'a ProjectCrates) -> Box<dyn BuildRoute + 'a> {
        let cargo =
            crates.program_target(&self.bin_stem, self.directory.clone(), self.binary.clone());
        let rustc = RustcRoute::new(self.directory.clone(), &self.crate_name, &self.binary);
        BuildRoutes::default().preferring_cargo(cargo, rustc)
    }
}
