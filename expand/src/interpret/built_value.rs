use super::builtin_site::BuiltinSite;
use super::limit_problems::LimitProblems;
use super::value_limits::ValueLimits;
use crate::diagnostic::Diagnostic;

pub trait BuiltValue: Sized {
    const LIMIT: usize;

    fn built_size(&self) -> usize;

    fn too_large(site: BuiltinSite, size: usize) -> Diagnostic;
}

impl BuiltValue for String {
    const LIMIT: usize = ValueLimits::TEXT_BYTES;

    fn built_size(&self) -> usize {
        self.len()
    }

    fn too_large(site: BuiltinSite, bytes: usize) -> Diagnostic {
        LimitProblems::text_too_long(site, bytes, Self::LIMIT)
    }
}
