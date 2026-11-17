use std::path::Path;

use crate::core::UnnamedPartPath;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::modules::{SearchScope, UnnamedPartTarget};
use crate::source::Span;

pub struct ModulePathProblems;

impl ModulePathProblems {
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
            UnnamedPartTarget::BeyondTaskReach(file) => Self::beyond_task_reach(diagnostic, &file),
            UnnamedPartTarget::Absent => {
                diagnostic.with_note("this path names no file under the root directory")
            }
        }
    }

    pub fn beyond_task_reach(diagnostic: Diagnostic, file: &Path) -> Diagnostic {
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
