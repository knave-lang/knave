use super::module_path_problems::ModulePathProblems;
use super::problems::ExpandProblems;
use crate::core::{ModulePath, UnnamedPartPath};
use crate::diagnostic::Diagnostic;
use crate::modules::LocalFiles;
use crate::source::Span;

pub(super) struct LocalFileDiagnosis<'s> {
    local_files: LocalFiles<'s>,
}

impl<'s> LocalFileDiagnosis<'s> {
    pub(super) fn new(local_files: LocalFiles<'s>) -> Self {
        Self { local_files }
    }

    pub(super) fn missing_module(&self, path: &ModulePath, span: Span) -> Diagnostic {
        match self.local_files.unreachable_file(path) {
            Some(file) => ExpandProblems::unreachable_module(path, &file, span),
            None => ExpandProblems::unknown_module(path, span),
        }
    }

    pub(super) fn unnamed_part_use(&self, path: &UnnamedPartPath, span: Span) -> Diagnostic {
        let target = self.local_files.unnamed_part_target(path);
        ModulePathProblems::unnamed_part(path, target, span)
    }
}
