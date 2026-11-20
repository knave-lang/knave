use rustc_hash::FxHashSet;

use super::body_walk::BodyWalk;
use super::loan_conflict::LoanConflict;
use super::loan_conflict_problems::LoanConflictProblems;
use super::loan_rules::LoanRules;
use crate::diagnostic::Diagnostic;
use crate::modules::ModuleSources;
use crate::source::Span;
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

    pub fn check(&self, reported: &[Span]) -> Vec<Diagnostic> {
        let mut changed = FxHashSet::default();
        self.user_functions()
            .flat_map(|lambda| self.body_conflicts(lambda, reported))
            .filter(|diagnostic| changed.insert(diagnostic.primary().span()))
            .collect()
    }

    fn is_reported(conflict: &LoanConflict<'_>, labels: &[Span]) -> bool {
        let spans = [conflict.access().span(), conflict.lending().span()];
        labels
            .iter()
            .any(|label| spans.iter().any(|span| label.contains(*span)))
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

    fn body_conflicts(&self, lambda: &'p TypedLambda, reported: &[Span]) -> Vec<Diagnostic> {
        let facts = BodyWalk::new(&self.rules).walk(lambda);
        let mut diagnostics: Vec<Diagnostic> = facts
            .conflicts()
            .iter()
            .filter(|conflict| !Self::is_reported(conflict, reported))
            .map(|conflict| {
                LoanConflictProblems::new(conflict, &self.program.bindings).diagnostic()
            })
            .collect();
        for closure in facts.closures() {
            diagnostics.extend(self.body_conflicts(closure.lambda(), reported));
        }
        diagnostics
    }
}
