use std::io;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::standard_error::StandardError;
use super::temporary_directory::TemporaryDirectory;
use super::terminal::{BuildRoute, BuildTarget, Terminal};
use crate::build::{BuildProfile, CargoTarget};
use crate::emit::EmittedCrate;

pub struct TestBinary {
    path: PathBuf,
    _directory: Option<TemporaryDirectory>,
}

impl TestBinary {
    const CRATE_NAME: &'static str = "tests";

    pub fn build(
        terminal: &Terminal,
        emitted: &EmittedCrate,
        project: Option<&LocatedProject>,
        profile: BuildProfile,
    ) -> Result<Self, ExitStatus> {
        let cargo = project.and_then(|project| project.crates().test_target(Self::CRATE_NAME));
        match cargo {
            Some(target) => Self::compile_with_cargo(terminal, emitted, target, profile),
            None => Self::compile_directly(terminal, emitted, profile),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn compile_with_cargo(
        terminal: &Terminal,
        emitted: &EmittedCrate,
        target: CargoTarget<'_>,
        profile: BuildProfile,
    ) -> Result<Self, ExitStatus> {
        let path = target.output.clone();
        terminal.compile_into(emitted, &BuildRoute::Cargo(target), profile)?;
        Ok(Self {
            path,
            _directory: None,
        })
    }

    fn compile_directly(
        terminal: &Terminal,
        emitted: &EmittedCrate,
        profile: BuildProfile,
    ) -> Result<Self, ExitStatus> {
        let directory = TemporaryDirectory::create().map_err(Self::report_no_build_directory)?;
        let path = directory.path().join(Self::CRATE_NAME);
        let target = BuildTarget {
            directory: directory.path().to_path_buf(),
            crate_name: Self::CRATE_NAME,
            output: &path,
        };
        terminal.compile_into(emitted, &BuildRoute::Rustc(target), profile)?;
        Ok(Self {
            path,
            _directory: Some(directory),
        })
    }

    fn report_no_build_directory(error: io::Error) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: cannot create a build directory: {error}"
        ));
        ExitStatus::Environment
    }
}
