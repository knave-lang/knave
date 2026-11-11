use super::builtin_site::BuiltinSite;
use super::limit_problems::LimitProblems;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyNotes {
    pub limit: usize,
}

impl TooManyNotes {
    pub fn at(self, function: &'static str, span: Span) -> Diagnostic {
        let site = BuiltinSite { function, span };
        LimitProblems::too_many_module_notes(site, self.limit + 1, self.limit)
    }
}
