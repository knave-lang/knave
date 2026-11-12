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
use crate::ownership::EnforcedOwnership;

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
        let emitted = terminal.emit_file::<EnforcedOwnership>(self.program.file())?;
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;

    use super::ProgramBinary;
    use crate::build::BuildProfile;
    use crate::cli::build_directory::BuildDirectory;
    use crate::cli::build_options::BuildOptions;
    use crate::cli::fake_programs::FakePrograms;
    use crate::cli::fake_stages::FakeStages;
    use crate::cli::located_program::LocatedProgram;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::temporary_directory::TemporaryDirectory;
    use crate::cli::terminal::Terminal;
    use crate::cli::terminal_settings::TerminalSettings;
    use crate::language::Language;

    struct ProgramBinaryFixture {
        root: TemporaryDirectory,
        terminal: Terminal,
    }

    impl ProgramBinaryFixture {
        const PROJECT: &str = "geometry";
        const PROJECT_FILE: &str = "(defproject geometry)\n";
        const ENTRY: &str = "src/main.knv";
        const CHOSEN_OUTPUT: &str = "chosen";

        fn new() -> Self {
            let root = TemporaryDirectory::create().expect("a temporary directory");
            fs::write(
                root.path().join(Language::project_file_name()),
                Self::PROJECT_FILE,
            )
            .expect("the project file is written");
            let entry = root.path().join(Self::ENTRY);
            fs::create_dir_all(entry.parent().expect("the entry has a directory"))
                .expect("the source directory is created");
            fs::write(&entry, FakePrograms::PRINTS_A_SUM).expect("the entry is written");
            Self {
                root,
                terminal: Terminal::new(TerminalSettings::reporting_to(Rc::new(
                    RecordedOutput::default(),
                ))),
            }
        }

        fn build(&self, output: Option<PathBuf>) -> PathBuf {
            let program =
                LocatedProgram::of_file(self.root.path().join(Self::ENTRY), &self.terminal)
                    .expect("the entry is located");
            let options = BuildOptions {
                profile: BuildProfile::Debug,
                output,
            };
            ProgramBinary::new(&program, &options)
                .compile(
                    &FakeStages::emitted(FakePrograms::PRINTS_A_SUM),
                    &self.terminal,
                )
                .expect("the entry builds")
        }

        fn project_binary(&self) -> PathBuf {
            self.root
                .path()
                .join(BuildDirectory::root())
                .join(Self::PROJECT)
                .join(Self::PROJECT)
        }

        fn chosen_output(&self) -> PathBuf {
            self.root.path().join(Self::CHOSEN_OUTPUT)
        }
    }

    #[test]
    fn builds_a_project_entry_under_the_project_build_directory() {
        let fixture = ProgramBinaryFixture::new();
        let binary = fixture.build(None);
        assert_eq!(binary, fixture.project_binary());
        assert!(binary.is_file());
    }

    #[test]
    fn builds_the_program_at_the_chosen_output() {
        let fixture = ProgramBinaryFixture::new();
        let chosen = fixture.chosen_output();
        let binary = fixture.build(Some(chosen.clone()));
        assert_eq!(binary, chosen);
        assert!(chosen.is_file());
    }
}
