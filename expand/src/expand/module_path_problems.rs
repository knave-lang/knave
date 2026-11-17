use crate::core::{DottedModulePath, ModulePath};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::modules::{DottedPathTarget, LocalFiles};
use crate::source::Span;

pub struct ModulePathProblems;

impl ModulePathProblems {
    pub fn dot_segment(
        path: &DottedModulePath,
        local_files: &LocalFiles,
        span: Span,
    ) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            DiagnosticCode::DOT_SEGMENT_IN_MODULE_PATH,
            format!("module path `{path}` has a `.` or `..` part"),
            Label::new(span, "a module path names a file from the root directory"),
        );
        match local_files.dotted_path_target(path) {
            DottedPathTarget::Reachable(written) => Self::write(diagnostic, &written),
            DottedPathTarget::Unreachable(file) => diagnostic.with_note(format!(
                "`{}` exists, but no module path this module can use names it",
                file.display()
            )),
            DottedPathTarget::Absent => match path.without_dot_parts() {
                Some(written) => Self::write(diagnostic, &written),
                None => diagnostic.with_note("this path names no file under the root directory"),
            },
        }
    }

    fn write(diagnostic: Diagnostic, written: &ModulePath) -> Diagnostic {
        diagnostic.with_help(format!("write `{written}`"))
    }
}
