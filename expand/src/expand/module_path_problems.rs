use std::path::Path;

use crate::core::{ModulePath, UnnamedPartPath};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::language::Language;
use crate::modules::{SearchScope, UnlistedLibrary, UnnamedPartTarget};
use crate::source::Span;

pub struct ModulePathProblems;

impl ModulePathProblems {
    pub fn file_of_another_module(kept: &ModulePath, refused: &ModulePath, at: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::FILE_OF_ANOTHER_MODULE,
            format!("module path `{refused}` loads the file of module `{kept}`"),
            Label::new(at, format!("this file is module `{kept}`")),
        )
        .with_help(format!("write `{kept}`"))
    }

    pub fn unnamed_part(
        path: &UnnamedPartPath,
        target: UnnamedPartTarget,
        span: Span,
    ) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            DiagnosticCode::UNNAMED_PART_IN_MODULE_PATH,
            format!("module path `{path}` has a part that is not a name"),
            Label::new(
                span,
                "each part of a module path names a directory or a file",
            ),
        );
        match target {
            UnnamedPartTarget::Reachable(written) => {
                diagnostic.with_help(format!("write `{written}`"))
            }
            UnnamedPartTarget::Unreachable(file) => diagnostic.with_note(format!(
                "`{}` exists, but no module path this module can use names it",
                file.display()
            )),
            UnnamedPartTarget::BeyondTaskReach(file) => {
                Self::with_task_reach_advice(diagnostic, &file)
            }
            UnnamedPartTarget::LibraryFile { file, unlisted } => {
                let diagnostic = diagnostic.with_note(format!(
                    "`{}` is a file of library `{}`",
                    file.display(),
                    unlisted.name
                ));
                Self::with_listing_advice(diagnostic, &unlisted)
            }
            UnnamedPartTarget::Absent => {
                diagnostic.with_note("this path names no file under the root directory")
            }
        }
    }

    pub fn with_listing_advice(diagnostic: Diagnostic, unlisted: &UnlistedLibrary) -> Diagnostic {
        diagnostic.with_help(format!(
            "add `{}` to the `{}` of `{}`",
            unlisted.name,
            unlisted.section,
            Language::project_file_name()
        ))
    }

    pub fn with_task_reach_advice(diagnostic: Diagnostic, file: &Path) -> Diagnostic {
        diagnostic
            .with_note(format!(
                "`{}` exists, under a directory a task cannot reach this way",
                file.display()
            ))
            .with_help(format!(
                "a task uses a module of `{tasks}/` by its path from `{tasks}/`, and cannot use a module under `{tests}/`",
                tasks = SearchScope::TASKS_DIRECTORY,
                tests = SearchScope::TESTS_DIRECTORY,
            ))
    }
}
