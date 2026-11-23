use rustc_hash::FxHashMap;

use super::settlement_failure::SettlementFailure;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::Span;

#[derive(Default)]
pub(super) struct SettlementVerdicts {
    refused: Vec<(Vec<Span>, Diagnostic)>,
    waiting: Diagnostics,
    agreed: FxHashMap<Span, String>,
}

impl SettlementVerdicts {
    pub(super) fn refuse(&mut self, reads: impl Iterator<Item = Span>, refusal: Diagnostic) {
        self.refused.push((reads.collect(), refusal));
    }

    pub(super) fn wait(&mut self, refusal: Diagnostic) {
        self.waiting.push(refusal);
    }

    pub(super) fn agree(&mut self, read: Span, shown: String) {
        self.agreed.insert(read, shown);
    }

    pub(super) fn refuses_any(&self, reads: &[Span]) -> bool {
        (self.refused.iter()).any(|(refused, _)| refused.iter().any(|read| reads.contains(read)))
    }

    pub(super) fn agreed_read(&self, reads: &[Span]) -> Option<&str> {
        (reads.iter()).find_map(|read| self.agreed.get(read).map(String::as_str))
    }

    pub(super) fn first_failure(&self) -> Option<SettlementFailure> {
        if !self.refused.is_empty() {
            let refusals = self.refused.iter().map(|(_, refusal)| refusal.clone());
            Some(SettlementFailure::Refused(refusals.collect()))
        } else {
            let waiting = (!self.waiting.is_empty()).then(|| self.waiting.clone());
            waiting.map(SettlementFailure::Waiting)
        }
    }
}
