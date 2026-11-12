use std::io;
use std::path::{Path, PathBuf};

use super::build_routes::BuildRoutes;
use super::crate_name::CrateName;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::rustc_route::RustcRoute;
use super::standard_error::StandardError;
use super::temporary_directory::TemporaryDirectory;
use super::terminal::Terminal;
use crate::build::BuildProfile;
use crate::emit::EmittedCrate;

pub struct TestBinary {
    path: PathBuf,
    _directory: TemporaryDirectory,
}

impl TestBinary {
    const CRATE_STEM: &'static str = "tests";

    pub fn build(
        terminal: &Terminal,
        emitted: &EmittedCrate,
        project: Option<&LocatedProject>,
        profile: BuildProfile,
    ) -> Result<Self, ExitStatus> {
        let directory = TemporaryDirectory::create().map_err(Self::report_no_build_directory)?;
        let crate_name = CrateName::of(Path::new(Self::CRATE_STEM));
        let path = crate_name.path_in(directory.path());
        let cargo = project.and_then(|project| project.crates().test_target(path.clone()));
        let rustc = RustcRoute::new(directory.path().to_path_buf(), &crate_name, &path);
        terminal.crate_build().compile(
            emitted,
            BuildRoutes::default()
                .preferring_cargo(cargo, rustc)
                .as_ref(),
            profile,
        )?;
        Ok(Self {
            path,
            _directory: directory,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn report_no_build_directory(error: io::Error) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: cannot create a build directory: {error}"
        ));
        ExitStatus::Environment
    }
}
