use rustc_hash::FxHashSet;

use super::field_owner_problems::FieldOwnerProblems;
use super::missing_annotations::MissingAnnotations;
use super::open_read_facts::OpenedRead;
use super::ty::{Type, TypeVar};
use super::type_variables::TypeVariables;
use super::unifier::Unifier;
use crate::diagnostic::Diagnostics;

pub(super) struct UnsettledReads<'u> {
    variables: TypeVariables<'u>,
    opened: Vec<OpenedRead>,
}

impl<'u> UnsettledReads<'u> {
    pub(super) fn new(unifier: &'u Unifier, opened: Vec<OpenedRead>) -> Self {
        Self {
            variables: TypeVariables::new(unifier),
            opened,
        }
    }

    pub(super) fn report(self, missing: MissingAnnotations) -> Diagnostics {
        let missing_roots = missing.roots(|var| self.roots(var));
        let mut reported = FxHashSet::default();
        let mut diagnostics = Diagnostics::default();
        for opened in &self.opened {
            let roots = self.read_roots(opened);
            if roots.iter().any(|root| missing_roots.contains(root))
                && reported.insert(opened.owner)
            {
                let label = &opened.label;
                diagnostics.push(FieldOwnerProblems::ambiguous(&label.name, label.span));
                reported.extend(roots);
            }
        }
        diagnostics.extend(missing.one_per_variable(reported, |var| self.roots(var)));
        diagnostics
    }

    fn read_roots(&self, opened: &OpenedRead) -> Vec<TypeVar> {
        let mut roots = self.roots(opened.owner);
        roots.extend(self.variables.free_in(&opened.read));
        roots
    }

    fn roots(&self, var: TypeVar) -> Vec<TypeVar> {
        let roots = self.variables.free_in(&Type::Var(var));
        if roots.is_empty() { vec![var] } else { roots }
    }
}
