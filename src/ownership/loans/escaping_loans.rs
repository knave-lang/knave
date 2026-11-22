use rustc_hash::FxHashSet;

use super::body_facts::BodyFacts;
use super::labeled_spans::LabeledSpans;
use crate::diagnostic::Diagnostic;
use crate::ownership::owned_values::OwnedValues;
use crate::ownership::problems::OwnershipProblems;

pub struct EscapingLoans<'b> {
    facts: &'b BodyFacts<'b>,
    owned: OwnedValues<'b>,
}

impl<'b> EscapingLoans<'b> {
    pub fn new(facts: &'b BodyFacts<'b>, owned: OwnedValues<'b>) -> Self {
        Self { facts, owned }
    }

    pub fn diagnostics(&self, labeled: &LabeledSpans) -> Vec<Diagnostic> {
        let mut dropped_spans = FxHashSet::default();
        self.facts
            .escapes()
            .into_iter()
            .filter_map(|lent| Some((lent, self.owned.of(lent.root())?)))
            .filter(|(lent, dropped)| !labeled.cover_any(&[lent.span(), dropped.span()]))
            .filter(|(_, dropped)| dropped_spans.insert(dropped.span()))
            .map(|(lent, dropped)| OwnershipProblems::local_borrow(lent.span(), &dropped))
            .collect()
    }
}
