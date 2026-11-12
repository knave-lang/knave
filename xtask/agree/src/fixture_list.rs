use std::fs;
use std::path::{Path, PathBuf};

use crate::program_source::ProgramSource;
use crate::subject_program::SubjectProgram;

pub struct FixtureList {
    paths: Vec<PathBuf>,
}

impl FixtureList {
    const ARGUMENTS_EXTENSION: &str = "arguments";
    const STDIN_EXTENSION: &str = "stdin";

    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    fn program(path: &Path) -> SubjectProgram {
        let text = fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("cannot read the fixture `{}`: {error}", path.display())
        });
        let file_name = PathBuf::from(path.file_name().unwrap_or_default());
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let mut program = SubjectProgram::of_entry(name, file_name, text);
        program.arguments = Self::arguments(path);
        program.stdin = fs::read(path.with_extension(Self::STDIN_EXTENSION)).unwrap_or_default();
        program
    }

    fn arguments(path: &Path) -> Vec<String> {
        fs::read_to_string(path.with_extension(Self::ARGUMENTS_EXTENSION))
            .map(|text| text.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }
}

impl ProgramSource for FixtureList {
    fn programs(&self) -> Box<dyn Iterator<Item = SubjectProgram> + '_> {
        Box::new(self.paths.iter().map(|path| Self::program(path)))
    }
}
