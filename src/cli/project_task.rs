use std::path::{Path, PathBuf};

use super::build_route::BuildRoute;
use super::build_routes::BuildRoutes;
use super::crate_name::CrateName;
use super::located_project::LocatedProject;
use super::rustc_route::RustcRoute;
use super::task_name::TaskName;
use super::task_target::TaskTarget;
use crate::modules::SearchScope;

pub struct ProjectTask<'p> {
    project: &'p LocatedProject,
    name: TaskName,
    binary: PathBuf,
    crate_name: CrateName,
}

impl<'p> ProjectTask<'p> {
    const BIN_INFIX: &'static str = "task";

    pub fn new(project: &'p LocatedProject, name: TaskName) -> Self {
        let directory = Self::directory_of(project, &name);
        Self {
            project,
            binary: directory.join(name.as_str()),
            crate_name: CrateName::of(Path::new(name.as_str())),
            name,
        }
    }

    fn directory_of(project: &LocatedProject, name: &TaskName) -> PathBuf {
        project
            .crates()
            .build_directory()
            .join(SearchScope::TASKS_DIRECTORY)
            .join(name.as_str())
    }

    pub fn build_directory(&self) -> PathBuf {
        Self::directory_of(self.project, &self.name)
    }

    pub fn root(&self) -> &Path {
        self.project.root()
    }
}

impl TaskTarget for ProjectTask<'_> {
    fn name(&self) -> &TaskName {
        &self.name
    }

    fn binary(&self) -> &Path {
        &self.binary
    }

    fn route(&self) -> Box<dyn BuildRoute + '_> {
        let bin_stem = format!(
            "{}-{}-{}",
            self.project.name(),
            Self::BIN_INFIX,
            self.name.as_str()
        );
        let cargo = self.project.crates().program_target(
            &bin_stem,
            self.build_directory(),
            self.binary.clone(),
        );
        let rustc = RustcRoute::new(self.build_directory(), &self.crate_name, &self.binary);
        BuildRoutes::default().preferring_cargo(cargo, rustc)
    }
}
