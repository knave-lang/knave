use crate::core::{DefinitionForm, KernelForm};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct ModuleLevelProblems;

impl ModuleLevelProblems {
    pub fn module_level_only(definer: &str, kernel: KernelForm, span: Span) -> Diagnostic {
        let misplaced = Diagnostic::error(
            DiagnosticCode::MODULE_LEVEL_ONLY,
            format!("`{definer}` can only appear at module level"),
            Label::new(span, "not allowed inside an expression"),
        );
        match kernel {
            KernelForm::Definition(DefinitionForm::Def) => {
                misplaced.with_help("use `let` to bind a name inside a body")
            }
            _ => misplaced,
        }
    }
}
