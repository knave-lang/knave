use rustc_hash::FxHashSet;

use super::body_facts::BodyFacts;
use super::body_walk::BodyWalk;
use super::labeled_spans::LabeledSpans;
use super::loan_rules::LoanRules;
use crate::diagnostic::Diagnostic;
use crate::modules::ModuleSources;
use crate::ownership::local_borrows::ParameterOwner;
use crate::ownership::owned_values::OwnedValues;
use crate::ownership::problems::OwnershipProblems;
use crate::types::{TypedExprKind, TypedLambda, TypedProgram};

pub struct LoanConflicts<'p> {
    program: &'p TypedProgram,
    rules: LoanRules<'p>,
}

impl<'p> LoanConflicts<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program,
            rules: LoanRules::of(program),
        }
    }

    pub fn check(&self, labeled: &LabeledSpans) -> Vec<Diagnostic> {
        let mut reported_spans = FxHashSet::default();
        self.user_functions()
            .flat_map(|lambda| self.body_conflicts(lambda, ParameterOwner::Function, labeled))
            .filter(|diagnostic| reported_spans.insert(diagnostic.primary().span()))
            .collect()
    }

    fn user_functions(&self) -> impl Iterator<Item = &'p TypedLambda> + '_ {
        self.program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.definitions)
            .filter_map(|definition| match &definition.value.kind {
                TypedExprKind::Fn(lambda) => Some(lambda.as_ref()),
                _ => None,
            })
    }

    fn body_conflicts(
        &self,
        lambda: &'p TypedLambda,
        owner: ParameterOwner,
        labeled: &LabeledSpans,
    ) -> Vec<Diagnostic> {
        let facts = BodyWalk::new(&self.rules).walk(lambda);
        let mut pairs = FxHashSet::default();
        let mut diagnostics: Vec<Diagnostic> = facts
            .conflicts()
            .iter()
            .filter(|conflict| !conflict.is_reported_in(labeled))
            .filter(|conflict| pairs.insert(conflict.spans()))
            .map(|conflict| conflict.diagnostic(&self.program.bindings))
            .collect();
        diagnostics.extend(Self::escapes(
            &facts,
            &OwnedValues::new(lambda, owner),
            labeled,
        ));
        for closure in facts.closures() {
            diagnostics.extend(self.body_conflicts(
                closure.lambda(),
                ParameterOwner::Closure,
                labeled,
            ));
        }
        diagnostics
    }

    fn escapes(
        facts: &BodyFacts<'_>,
        owned: &OwnedValues<'_>,
        labeled: &LabeledSpans,
    ) -> Vec<Diagnostic> {
        let mut dropped_spans = FxHashSet::default();
        facts
            .escapes()
            .into_iter()
            .filter_map(|lent| Some((lent, owned.of(lent.root())?)))
            .filter(|(lent, dropped)| !labeled.cover_any(&[lent.span(), dropped.span()]))
            .filter(|(_, dropped)| dropped_spans.insert(dropped.span()))
            .map(|(lent, dropped)| OwnershipProblems::local_borrow(lent.span(), &dropped))
            .collect()
    }
}
