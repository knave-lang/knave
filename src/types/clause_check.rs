use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::clause_problems::ClauseProblems;
use super::display::TypeDisplay;
use super::selection::Selection;
use super::template_clauses::ClauseTarget;
use super::traits::TraitTable;
use super::ty::{Bound, GenericId, Scheme, Type};
use crate::core::{RustClause, RustTemplate};
use crate::diagnostic::Diagnostic;

pub(super) struct ClauseCheck<'c> {
    traits: &'c TraitTable,
    display: TypeDisplay<'c>,
    open: Type,
}

impl<'c> ClauseCheck<'c> {
    pub(super) fn new(traits: &'c TraitTable, display: TypeDisplay<'c>, open: Type) -> Self {
        Self {
            traits,
            display,
            open,
        }
    }

    pub(super) fn problems(
        &self,
        scheme: &Scheme,
        template: &RustTemplate,
        clauses: &[(ClauseTarget, Rc<str>)],
    ) -> Vec<Diagnostic> {
        let mut problems = self.covered_clauses(template, clauses);
        for (written, (target, _)) in template.clauses.iter().zip(clauses) {
            problems.extend(self.dropped_borrows(scheme, written, target));
        }
        problems
    }

    fn covered_clauses(
        &self,
        template: &RustTemplate,
        clauses: &[(ClauseTarget, Rc<str>)],
    ) -> Vec<Diagnostic> {
        clauses
            .iter()
            .enumerate()
            .filter_map(|(index, (target, _))| {
                let covering = clauses[..index]
                    .iter()
                    .position(|(earlier, _)| earlier.covers(target, self.traits))?;
                let clause = template.clauses[index].target.span();
                let earlier = template.clauses[covering].target.span();
                Some(if clauses[covering].0 == *target {
                    ClauseProblems::repeated_clause(clause, earlier)
                } else {
                    ClauseProblems::covered_clause(clause, earlier)
                })
            })
            .collect()
    }

    fn dropped_borrows(
        &self,
        scheme: &Scheme,
        written: &RustClause,
        target: &ClauseTarget,
    ) -> Vec<Diagnostic> {
        let clause = written.target.span();
        written
            .dropped_borrows
            .arguments()
            .filter_map(|argument| {
                let ClauseTarget::Type(target) = target else {
                    return Some(ClauseProblems::trait_clause_drops_a_borrow(
                        argument, clause,
                    ));
                };
                match self.argument_type(scheme, target, argument)? {
                    Type::Ref(..) => None,
                    passed => Some(ClauseProblems::clause_drops_a_borrow(
                        argument,
                        &self.display.show(&passed),
                        clause,
                    )),
                }
            })
            .collect()
    }

    fn argument_type(&self, scheme: &Scheme, target: &Type, argument: usize) -> Option<Type> {
        let Type::Function(_, parameters, _) = &scheme.body else {
            return None;
        };
        let solved = self.solved_parameters(scheme, target)?;
        Some(parameters.get(argument)?.substitute(&solved))
    }

    fn solved_parameters(
        &self,
        scheme: &Scheme,
        target: &Type,
    ) -> Option<FxHashMap<GenericId, Type>> {
        let chooser = scheme.clause_chooser()?;
        let mut solved = FxHashMap::default();
        solved.insert(chooser.id, target.clone());
        for bound in &chooser.bounds {
            if let Some(Type::Generic(other)) = bound.argument
                && let Some(argument) = self.settled_argument(bound, target)
            {
                solved.insert(other, argument);
            }
        }
        Some(solved)
    }

    fn settled_argument(&self, bound: &Bound, target: &Type) -> Option<Type> {
        let open = Bound {
            argument: Some(self.open.clone()),
            ..bound.clone()
        };
        match self
            .traits
            .rows(bound.binding)
            .settle_lone_row(&open, target)?
        {
            Selection::Implemented { argument, .. } => argument,
            Selection::Undecided | Selection::Missing => None,
        }
    }
}
