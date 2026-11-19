use super::compiler_version_problems::CompilerVersionProblems;
use super::release_version::ReleaseVersion;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

#[derive(Debug)]
pub struct CompilerVersion {
    pub version: ReleaseVersion,
    pub span: Span,
}

impl CompilerVersion {
    pub fn problem(&self, running: ReleaseVersion) -> Option<Diagnostic> {
        self.refusal(running).or_else(|| {
            (self.version < running)
                .then(|| CompilerVersionProblems::newer_compiler(self.version, running, self.span))
        })
    }

    pub fn refusal(&self, running: ReleaseVersion) -> Option<Diagnostic> {
        (self.version > running)
            .then(|| CompilerVersionProblems::older_compiler(self.version, running, self.span))
    }
}
