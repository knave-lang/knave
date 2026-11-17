use crate::core::ModulePath;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct ModulePathProblems;

impl ModulePathProblems {
    pub fn dot_segment(path: &ModulePath, span: Span) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            DiagnosticCode::DOT_SEGMENT_IN_MODULE_PATH,
            format!("module path `{path}` has a `.` or `..` part"),
            Label::new(span, "a module path names a file from the root directory"),
        );
        match path.from_root() {
            Some(rooted) => diagnostic.with_help(format!("write `{rooted}`")),
            None => diagnostic.with_note("this path names no file under the root directory"),
        }
    }
}
