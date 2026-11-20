use rustc_hash::FxHashSet;

use super::pinning::InstantiationReport;
use super::ty::{GenericId, Type};
use super::unborrowed::Unborrowed;
use crate::core::BindingId;

pub(super) struct ReachingStructs<'r> {
    report: &'r InstantiationReport,
    visited: FxHashSet<(BindingId, GenericId)>,
}

impl<'r> ReachingStructs<'r> {
    pub(super) fn new(report: &'r InstantiationReport) -> Self {
        Self {
            report,
            visited: FxHashSet::default(),
        }
    }

    pub(super) fn of(
        mut self,
        binding: BindingId,
        generic: GenericId,
    ) -> Option<Vec<(BindingId, Vec<Type>)>> {
        self.collect(binding, generic)
    }

    fn collect(
        &mut self,
        binding: BindingId,
        generic: GenericId,
    ) -> Option<Vec<(BindingId, Vec<Type>)>> {
        let mut structs = Vec::new();
        for (enclosing, used) in self.report.sited_uses_of(binding, generic) {
            match used.as_ref().map(Unborrowed::of) {
                Some(Type::Named(owner, arguments)) => structs.push((*owner, arguments.clone())),
                Some(Type::Generic(caller)) => {
                    if self.visited.insert((enclosing, *caller)) {
                        structs.extend(self.collect(enclosing, *caller)?);
                    }
                }
                _ => return None,
            }
        }
        Some(structs)
    }
}
