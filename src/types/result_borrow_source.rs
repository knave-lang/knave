use super::lifetime_problems::LifetimeProblems;
use crate::core::{TypeExpr, TypeExprKind};
use crate::diagnostic::Diagnostic;

pub struct ResultBorrowSource<'t> {
    parameters: &'t [TypeExpr],
    result: &'t TypeExpr,
}

struct WrittenReferences {
    named: usize,
    elided: usize,
}

impl<'t> ResultBorrowSource<'t> {
    pub fn of(written: &'t TypeExpr) -> Option<Self> {
        match written.kind() {
            TypeExprKind::Where(_, inner) => Self::of(inner),
            TypeExprKind::Function(_, parameters, result) => Some(Self { parameters, result }),
            _ => None,
        }
    }

    pub fn check(&self) -> Result<(), Diagnostic> {
        if self.is_named() {
            return Ok(());
        }
        Err(LifetimeProblems::unnamed_result_borrow(self.result.span()))
    }

    fn is_named(&self) -> bool {
        let inputs: usize = self
            .parameters
            .iter()
            .map(|parameter| WrittenReferences::of(parameter).count())
            .sum();
        WrittenReferences::of(self.result).elided == 0 || inputs <= 1
    }
}

impl WrittenReferences {
    fn of(written: &TypeExpr) -> Self {
        let mut references = Self {
            named: 0,
            elided: 0,
        };
        references.add(written);
        references
    }

    fn count(&self) -> usize {
        self.named + self.elided
    }

    fn add(&mut self, written: &TypeExpr) {
        match written.kind() {
            TypeExprKind::Reference(_, Some(_), inner) => {
                self.named += 1;
                self.add(inner);
            }
            TypeExprKind::Reference(_, None, inner) => {
                self.elided += 1;
                self.add(inner);
            }
            TypeExprKind::Named(_, arguments) => arguments.iter().for_each(|part| self.add(part)),
            TypeExprKind::Where(_, inner) => self.add(inner),
            TypeExprKind::With(inner, _) => self.add(inner),
            TypeExprKind::Primitive(_)
            | TypeExprKind::Function(..)
            | TypeExprKind::Hole
            | TypeExprKind::Trait(_) => {}
        }
    }
}
