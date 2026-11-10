use std::io;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::standard_error::StandardError;
use super::temporary_directory::TemporaryDirectory;
use super::terminal::{BuildRoute, BuildTarget, Terminal};
use crate::build::BuildProfile;
use crate::emit::EmittedCrate;

pub struct TestBinary {
    path: PathBuf,
    _directory: TemporaryDirectory,
}

impl TestBinary {
    const CRATE_NAME: &'static str = "tests";

    pub fn build(
        terminal: &Terminal,
        emitted: &EmittedCrate,
        project: Option<&LocatedProject>,
        profile: BuildProfile,
    ) -> Result<Self, ExitStatus> {
        let directory = TemporaryDirectory::create().map_err(Self::report_no_build_directory)?;
        let path = directory.path().join(Self::CRATE_NAME);
        let build = Self::route(project, &directory, &path);
        terminal.compile_into(emitted, &build, profile)?;
        Ok(Self {
            path,
            _directory: directory,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn route<'b>(
        project: Option<&'b LocatedProject>,
        directory: &TemporaryDirectory,
        path: &'b Path,
    ) -> BuildRoute<'b> {
        let cargo = project.and_then(|project| project.crates().test_target(path.to_path_buf()));
        match cargo {
            Some(target) => BuildRoute::Cargo(target),
            None => BuildRoute::Rustc(BuildTarget {
                directory: directory.path().to_path_buf(),
                crate_name: Self::CRATE_NAME,
                output: path,
            }),
        }
    }

    fn report_no_build_directory(error: io::Error) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: cannot create a build directory: {error}"
        ));
        ExitStatus::Environment
    }
}
