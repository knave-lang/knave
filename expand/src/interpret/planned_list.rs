use super::built_value::BuiltValue;
use super::builtin_site::BuiltinSite;
use super::limit_problems::LimitProblems;
use super::value_limits::ValueLimits;
use crate::diagnostic::Diagnostic;

pub struct PlannedList(pub usize);

impl BuiltValue for PlannedList {
    const LIMIT: usize = ValueLimits::LIST_VALUES;

    fn built_size(&self) -> usize {
        self.0
    }

    fn too_large(site: BuiltinSite, size: usize) -> Diagnostic {
        LimitProblems::built_list_too_large(site, size, Self::LIMIT)
    }
}
