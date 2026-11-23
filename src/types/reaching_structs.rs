use rustc_hash::FxHashSet;

use super::pinning::InstantiationReport;
use super::ty::{GenericId, Type};
use super::unborrowed::Unborrowed;
use crate::core::BindingId;

type ReachedStruct = (BindingId, Vec<Type>);

pub(super) struct ReachingStructs<'r> {
    report: &'r InstantiationReport,
    visited: FxHashSet<(BindingId, GenericId)>,
    structs: Vec<ReachedStruct>,
    reached_by_other: bool,
}

impl<'r> ReachingStructs<'r> {
    pub(super) fn new(report: &'r InstantiationReport) -> Self {
        Self {
            report,
            visited: FxHashSet::default(),
            structs: Vec::new(),
            reached_by_other: false,
        }
    }

    pub(super) fn of(
        mut self,
        binding: BindingId,
        generic: GenericId,
    ) -> Option<Vec<ReachedStruct>> {
        self.walk(binding, generic);
        (!self.reached_by_other).then_some(self.structs)
    }

    pub(super) fn generic_callers(
        mut self,
        binding: BindingId,
        generic: GenericId,
    ) -> Vec<GenericId> {
        self.walk(binding, generic);
        self.visited.into_iter().map(|(_, caller)| caller).collect()
    }

    fn walk(&mut self, binding: BindingId, generic: GenericId) {
        for (enclosing, used) in self.report.sited_uses_of(binding, generic) {
            match used.as_ref().map(Unborrowed::of) {
                Some(Type::Named(owner, arguments)) => {
                    self.structs.push((*owner, arguments.clone()))
                }
                Some(Type::Generic(caller)) => {
                    if self.visited.insert((enclosing, *caller)) {
                        self.walk(enclosing, *caller);
                    }
                }
                _ => self.reached_by_other = true,
            }
        }
    }
}
