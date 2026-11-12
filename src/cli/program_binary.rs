use std::path::{Path, PathBuf};

use super::build_directory::BuildDirectory;
use super::build_options::BuildOptions;
use super::build_route::BuildRoute;
use super::build_routes::BuildRoutes;
use super::crate_name::CrateName;
use super::exit_status::ExitStatus;
use super::located_program::LocatedProgram;
use super::program_file::ProgramFile;
use super::rustc_route::RustcRoute;
use super::terminal::Terminal;
use super::working_directory::WorkingDirectory;
use crate::emit::EmittedCrate;

pub struct ProgramBinary<'p> {
    program: &'p LocatedProgram,
    options: &'p BuildOptions,
}

struct BinaryPlace {
    directory: PathBuf,
    binary: PathBuf,
}

impl<'p> ProgramBinary<'p> {
    pub fn new(program: &'p LocatedProgram, options: &'p BuildOptions) -> Self {
        Self { program, options }
    }

    pub fn build(&self, terminal: &mut Terminal) -> Result<PathBuf, ExitStatus> {
        let emitted = terminal.emit_file(self.program.file())?;
        self.compile(&emitted, terminal)
    }

    pub fn compile(
        &self,
        emitted: &EmittedCrate,
        terminal: &Terminal,
    ) -> Result<PathBuf, ExitStatus> {
        let crate_name = CrateName::of(self.program.file().path());
        let place = self.place(&crate_name)?;
        let output = place.output(self.options.output.as_ref());
        terminal.crate_build().compile(
            emitted,
            self.route(&crate_name, place.into_directory(), &output)
                .as_ref(),
            self.options.profile,
        )?;
        Ok(output)
    }

    fn place(&self, crate_name: &CrateName) -> Result<BinaryPlace, ExitStatus> {
        match self.program.entry_project() {
            Some(project) => Ok(BinaryPlace::named(
                project.build_directory(),
                project.name(),
            )),
            None => self.entry_place(crate_name),
        }
    }

    fn entry_place(&self, crate_name: &CrateName) -> Result<BinaryPlace, ExitStatus> {
        let working_directory = WorkingDirectory::resolved()?;
        let entry_directory =
            WorkingDirectory::resolve(&ProgramFile::directory_of(self.program.file().path()))?;
        let directory = BuildDirectory::new(BuildDirectory::root(), working_directory)
            .for_entry(&entry_directory, crate_name);
        let binary = crate_name.path_in(&directory);
        Ok(BinaryPlace { directory, binary })
    }

    fn route<'r>(
        &'r self,
        crate_name: &'r CrateName,
        directory: PathBuf,
        output: &'r Path,
    ) -> Box<dyn BuildRoute + 'r> {
        let cargo = self.program.project().and_then(|project| {
            project
                .crates()
                .program_target(project.name(), directory.clone(), output.to_path_buf())
        });
        BuildRoutes::default()
            .preferring_cargo(cargo, RustcRoute::new(directory, crate_name, output))
    }
}

impl BinaryPlace {
    fn named(directory: PathBuf, name: &str) -> Self {
        let binary = directory.join(name);
        Self { directory, binary }
    }

    fn output(&self, chosen: Option<&PathBuf>) -> PathBuf {
        chosen.unwrap_or(&self.binary).clone()
    }

    fn into_directory(self) -> PathBuf {
        self.directory
    }
}
