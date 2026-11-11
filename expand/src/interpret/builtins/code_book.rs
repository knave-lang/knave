use std::rc::Rc;

use crate::diagnostic::{CodeStage, Diagnostic, DiagnosticCode, SimilarName};
use crate::interpret::problems::InterpretProblems;
use crate::interpret::warning_problems::WarningProblems;
use crate::source::Span;

type UnknownCode = fn(&str, Span, Option<Rc<str>>) -> Diagnostic;

pub struct CodeBook {
    stage: CodeStage,
    unknown: UnknownCode,
}

impl CodeBook {
    pub const MACROS: Self = Self {
        stage: CodeStage::MACROS,
        unknown: InterpretProblems::unknown_failure_code,
    };
    pub const LINTS: Self = Self {
        stage: CodeStage::LINTS,
        unknown: WarningProblems::unknown_lint,
    };

    pub fn code_named(&self, name: &str, span: Span) -> Result<DiagnosticCode, Diagnostic> {
        self.stage.code_named(name).ok_or_else(|| {
            let similar = SimilarName::closest(name, &self.stage.names());
            (self.unknown)(name, span, similar)
        })
    }
}
