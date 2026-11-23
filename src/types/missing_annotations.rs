use rustc_hash::FxHashSet;

use super::ty::TypeVar;
use crate::diagnostic::{Diagnostic, Diagnostics};

struct MissingAnnotation {
    open_vars: Vec<TypeVar>,
    diagnostic: Diagnostic,
}

#[derive(Default)]
pub struct MissingAnnotations {
    reports: Vec<MissingAnnotation>,
}

impl MissingAnnotations {
    pub fn defer(&mut self, open_vars: Vec<TypeVar>, diagnostic: Diagnostic) {
        self.reports.push(MissingAnnotation {
            open_vars,
            diagnostic,
        });
    }

    pub fn roots(&self, roots_of: impl Fn(TypeVar) -> Vec<TypeVar>) -> FxHashSet<TypeVar> {
        let vars = self.reports.iter().flat_map(|report| &report.open_vars);
        vars.flat_map(|var| roots_of(*var)).collect()
    }

    pub fn one_per_variable(
        self,
        mut reported_roots: FxHashSet<TypeVar>,
        roots_of: impl Fn(TypeVar) -> Vec<TypeVar>,
    ) -> Diagnostics {
        let mut diagnostics = Diagnostics::default();
        for report in self.reports {
            let roots = report.open_vars.iter().flat_map(|var| roots_of(*var));
            let new_roots = roots.filter(|root| reported_roots.insert(*root));
            if new_roots.count() > 0 {
                diagnostics.push(report.diagnostic);
            }
        }
        diagnostics
    }
}
