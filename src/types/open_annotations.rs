use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::bound_origins::BoundOrigins;
use super::field_bound_problems::FieldBoundProblems;
use super::held_bound::HeldBound;
use super::missing_annotations::MissingAnnotations;
use super::open_read_facts::OpenReadFacts;
use super::open_round_end::OpenRoundEnd;
use super::settled_read::SettledRead;
use super::ty::TypeVar;
use super::unifier::Unifier;
use super::unsettled_reads::UnsettledReads;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::Span;

#[derive(Default)]
pub(super) struct OpenAnnotations {
    missing: MissingAnnotations,
    reads: OpenReadFacts,
    held: Vec<HeldBound>,
}

impl OpenAnnotations {
    pub(super) fn new(settled: Rc<FxHashMap<Span, SettledRead>>) -> Self {
        Self {
            missing: MissingAnnotations::default(),
            reads: OpenReadFacts::new(settled),
            held: Vec::new(),
        }
    }

    pub(super) fn defer(&mut self, open_vars: Vec<TypeVar>, diagnostic: Diagnostic) {
        self.missing.defer(open_vars, diagnostic);
    }

    pub(super) fn hold(
        &mut self,
        problems: FieldBoundProblems,
        read: String,
        origins: BoundOrigins,
    ) {
        self.held.push(HeldBound::new(problems, read, origins));
    }

    pub(super) fn reads_mut(&mut self) -> &mut OpenReadFacts {
        &mut self.reads
    }

    pub(super) fn take_requested(&mut self) -> FxHashSet<Rc<str>> {
        self.reads.take_requested()
    }

    pub(super) fn end_round(&mut self, unifier: &Unifier, earlier: &Diagnostics) -> OpenRoundEnd {
        let held = std::mem::take(&mut self.held);
        let opened = self.reads.take_opened();
        let missing = std::mem::take(&mut self.missing);
        let missing = if earlier.has_errors() || !held.is_empty() {
            Diagnostics::default()
        } else {
            UnsettledReads::new(unifier, opened).report(missing)
        };
        OpenRoundEnd { missing, held }
    }
}
