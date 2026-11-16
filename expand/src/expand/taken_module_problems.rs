use crate::core::ModulePath;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::language::Language;
use crate::modules::LibraryTable;
use crate::source::Span;

pub struct TakenModuleProblems;

impl TakenModuleProblems {
    pub fn file_takes_compiler_module(path: &ModulePath, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::FILE_TAKES_COMPILER_MODULE,
            format!("this file's module path `{path}` belongs to the compiler"),
            Label::new(span, "this file never loads"),
        )
        .with_note("`prelude`, `kernel`, `code` and every path under `prelude/` are the compiler's, so this file would never run")
        .with_help("rename the file")
    }

    pub fn file_takes_library_module(path: &ModulePath, span: Span) -> Diagnostic {
        let library = LibraryTable::head(path);
        Diagnostic::error(
            DiagnosticCode::FILE_TAKES_LIBRARY_MODULE,
            format!("library `{library}` takes this file's module path `{path}`"),
            Label::new(span, "this file never loads"),
        )
        .with_note("a module path that starts with a library's name loads from the library, so this file would never run")
        .with_help(format!(
            "rename the file, or rename the library in `{}`",
            Language::project_file_name()
        ))
    }
}
