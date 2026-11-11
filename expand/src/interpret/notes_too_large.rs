use super::builtin_site::BuiltinSite;
use super::limit_problems::LimitProblems;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotesTooLarge {
    pub held_bytes: usize,
    pub limit: usize,
}

impl NotesTooLarge {
    pub fn at(self, function: &'static str, span: Span) -> Diagnostic {
        let site = BuiltinSite { function, span };
        LimitProblems::module_notes_too_large(site, self.held_bytes, self.limit)
    }
}
