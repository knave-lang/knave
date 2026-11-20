use rustc_hash::FxHashSet;

use super::body_walk::BodyWalk;
use super::labeled_spans::LabeledSpans;
use super::loan_rules::LoanRules;
use crate::diagnostic::Diagnostic;
use crate::modules::ModuleSources;
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
            .flat_map(|lambda| self.body_conflicts(lambda, labeled))
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

    fn body_conflicts(&self, lambda: &'p TypedLambda, labeled: &LabeledSpans) -> Vec<Diagnostic> {
        let facts = BodyWalk::new(&self.rules).walk(lambda);
        let mut pairs = FxHashSet::default();
        let mut diagnostics: Vec<Diagnostic> = facts
            .conflicts()
            .iter()
            .filter(|conflict| !conflict.is_reported_in(labeled))
            .filter(|conflict| pairs.insert(conflict.spans()))
            .map(|conflict| conflict.diagnostic(&self.program.bindings))
            .collect();
        for closure in facts.closures() {
            diagnostics.extend(self.body_conflicts(closure.lambda(), labeled));
        }
        diagnostics
    }
}
