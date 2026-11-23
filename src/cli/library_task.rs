use std::path::{Path, PathBuf};

use super::build_route::BuildRoute;
use super::located_project::LocatedProject;
use super::project_crates::ProjectCrates;
use super::task_build::TaskBuild;
use super::task_name::TaskName;
use super::task_target::TaskTarget;

pub struct LibraryTask {
    name: TaskName,
    build: TaskBuild,
    crates: ProjectCrates,
    root: PathBuf,
}

pub struct LibraryTaskSite<'s> {
    pub dependent: &'s LocatedProject,
    pub library_name: &'s str,
    pub task: &'s str,
}

impl LibraryTask {
    const DIRECTORY: &'static str = "library_tasks";
    const BIN_INFIX: &'static str = "library-task";

    pub fn new(site: &LibraryTaskSite<'_>, library: &LocatedProject, name: TaskName) -> Self {
        let library_directory = site
            .dependent
            .crates()
            .build_directory()
            .join(Self::DIRECTORY)
            .join(site.library_name);
        let bin_stem = format!(
            "{}-{}-{}-{}",
            site.dependent.name(),
            Self::BIN_INFIX,
            site.library_name,
            name.as_str()
        );
        Self {
            build: TaskBuild::new(bin_stem, library_directory.join(name.as_str()), &name),
            crates: library.crates().locked_at(&library_directory),
            root: site.dependent.root().to_path_buf(),
            name,
        }
    }

    pub fn crates(&self) -> &ProjectCrates {
        &self.crates
    }
}

impl TaskTarget for LibraryTask {
    fn name(&self) -> &TaskName {
        &self.name
    }

    fn binary(&self) -> &Path {
        self.build.binary()
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn route(&self) -> Box<dyn BuildRoute + '_> {
        self.build.route(&self.crates)
    }
}
