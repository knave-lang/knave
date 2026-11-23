use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::build_route::BuildRoute;
use super::build_routes::BuildRoutes;
use super::crate_name::CrateName;
use super::located_project::LocatedProject;
use super::project_crates::ProjectCrates;
use super::rustc_route::RustcRoute;
use super::task_name::TaskName;
use super::task_target::TaskTarget;

pub struct LibraryTask {
    name: TaskName,
    bin_stem: String,
    directory: PathBuf,
    binary: PathBuf,
    crate_name: CrateName,
    crates: ProjectCrates,
}

pub struct LibraryTaskSite<'s> {
    pub dependent: &'s LocatedProject,
    pub library: &'s LocatedProject,
    pub library_name: &'s str,
}

impl LibraryTask {
    const DIRECTORY: &'static str = "library_tasks";
    const BIN_INFIX: &'static str = "library-task";
    const LOCK_FILE: &'static str = "Cargo.lock";

    pub fn new(site: &LibraryTaskSite<'_>, name: TaskName) -> Self {
        let library_directory = site
            .dependent
            .crates()
            .build_directory()
            .join(Self::DIRECTORY)
            .join(site.library_name);
        let directory = library_directory.join(name.as_str());
        Self {
            bin_stem: format!(
                "{}-{}-{}-{}",
                site.dependent.name(),
                Self::BIN_INFIX,
                site.library_name,
                name.as_str()
            ),
            binary: directory.join(name.as_str()),
            crate_name: CrateName::of(Path::new(name.as_str())),
            crates: site
                .library
                .crates()
                .locked_at(library_directory.join(Self::LOCK_FILE)),
            directory,
            name,
        }
    }

    pub fn copy_lock_from(&self, library: &LocatedProject) -> io::Result<()> {
        let lock = library.root().join(Self::LOCK_FILE);
        if !lock.is_file() {
            return Ok(());
        }
        let copy = self.crates.lock_file();
        if let Some(folder) = copy.parent() {
            fs::create_dir_all(folder)?;
        }
        fs::copy(lock, copy).map(|_| ())
    }
}

impl TaskTarget for LibraryTask {
    fn name(&self) -> &TaskName {
        &self.name
    }

    fn binary(&self) -> &Path {
        &self.binary
    }

    fn route(&self) -> Box<dyn BuildRoute + '_> {
        let cargo =
            self.crates
                .program_target(&self.bin_stem, self.directory.clone(), self.binary.clone());
        let rustc = RustcRoute::new(self.directory.clone(), &self.crate_name, &self.binary);
        BuildRoutes::default().preferring_cargo(cargo, rustc)
    }
}
