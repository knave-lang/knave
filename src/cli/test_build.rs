use std::io;
use std::path::PathBuf;

use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::standard_error::StandardError;
use super::temporary_directory::TemporaryDirectory;
use super::terminal::{BuildTarget, Terminal};
use crate::build::BuildProfile;
use crate::emit::EmittedCrate;

pub enum TestBuild<'p> {
    Direct(TemporaryDirectory),
    Cargo(&'p LocatedProject),
}

impl<'p> TestBuild<'p> {
    const CRATE_NAME: &'static str = "tests";

    pub fn for_project(project: Option<&'p LocatedProject>) -> Result<Self, ExitStatus> {
        match project.filter(|project| project.tests_use_crates()) {
            Some(project) => Ok(Self::Cargo(project)),
            None => TemporaryDirectory::create()
                .map(Self::Direct)
                .map_err(Self::report_no_build_directory),
        }
    }

    pub fn compile(
        &self,
        terminal: &Terminal,
        emitted: &EmittedCrate,
        profile: BuildProfile,
    ) -> Result<PathBuf, ExitStatus> {
        match self {
            Self::Direct(directory) => {
                Self::compile_directly(terminal, emitted, directory, profile)
            }
            Self::Cargo(project) => Self::compile_with_cargo(terminal, emitted, project, profile),
        }
    }

    fn compile_directly(
        terminal: &Terminal,
        emitted: &EmittedCrate,
        directory: &TemporaryDirectory,
        profile: BuildProfile,
    ) -> Result<PathBuf, ExitStatus> {
        let binary = directory.path().join(Self::CRATE_NAME);
        let target = BuildTarget {
            directory: directory.path().to_path_buf(),
            crate_name: Self::CRATE_NAME,
            output: &binary,
        };
        terminal.compile(emitted, &target, profile)?;
        Ok(binary)
    }

    fn compile_with_cargo(
        terminal: &Terminal,
        emitted: &EmittedCrate,
        project: &LocatedProject,
        profile: BuildProfile,
    ) -> Result<PathBuf, ExitStatus> {
        let directory = project.test_crate_directory();
        let binary = directory.join(Self::CRATE_NAME);
        let target = project.cargo_test_target(&binary);
        terminal.compile_with_cargo(emitted, &directory, &target, profile)?;
        Ok(binary)
    }

    fn report_no_build_directory(error: io::Error) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: cannot create a build directory: {error}"
        ));
        ExitStatus::Environment
    }
}
