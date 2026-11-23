use std::path::Path;

use super::build_route::BuildRoute;
use super::located_project::LocatedProject;
use super::task_build::TaskBuild;
use super::task_name::TaskName;
use super::task_target::TaskTarget;
use crate::modules::SearchScope;

pub struct ProjectTask<'p> {
    project: &'p LocatedProject,
    name: TaskName,
    build: TaskBuild,
}

impl<'p> ProjectTask<'p> {
    const BIN_INFIX: &'static str = "task";

    pub fn new(project: &'p LocatedProject, name: TaskName) -> Self {
        let directory = project
            .crates()
            .build_directory()
            .join(SearchScope::TASKS_DIRECTORY)
            .join(name.as_str());
        let bin_stem = format!("{}-{}-{}", project.name(), Self::BIN_INFIX, name.as_str());
        Self {
            project,
            build: TaskBuild::new(bin_stem, directory, &name),
            name,
        }
    }
}

impl TaskTarget for ProjectTask<'_> {
    fn name(&self) -> &TaskName {
        &self.name
    }

    fn binary(&self) -> &Path {
        self.build.binary()
    }

    fn root(&self) -> &Path {
        self.project.root()
    }

    fn route(&self) -> Box<dyn BuildRoute + '_> {
        self.build.route(self.project.crates())
    }
}
