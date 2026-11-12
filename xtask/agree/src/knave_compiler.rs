use std::path::PathBuf;

use crate::knave_invocation::KnaveInvocation;
use crate::scratch_directory::ScratchDirectory;
use crate::subject_program::SubjectProgram;

#[derive(Clone, Debug)]
pub struct KnaveCompiler {
    binary: PathBuf,
}

impl KnaveCompiler {
    pub fn at(binary: PathBuf) -> Self {
        Self { binary }
    }

    pub fn invocation<'a>(
        &'a self,
        program: &'a SubjectProgram,
        directory: &'a ScratchDirectory,
    ) -> KnaveInvocation<'a> {
        KnaveInvocation::new(&self.binary, program, directory)
    }
}
