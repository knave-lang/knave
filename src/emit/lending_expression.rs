use std::convert::Infallible;
use std::ops::ControlFlow;

use super::loan::Loan;
use super::temporary_scope::TemporaryScope;
use crate::types::{
    Borrow, TypedArgument, TypedControl, TypedExpr, TypedExprKind, TypedStructExpr,
};

#[derive(Clone, Copy)]
pub struct LendingExpression<'e> {
    expr: &'e TypedExpr,
}

impl<'e> LendingExpression<'e> {
    pub fn new(expr: &'e TypedExpr) -> Self {
        Self { expr }
    }

    pub fn may_lend(self) -> bool {
        self.try_each_loan(TemporaryScope::Statement, &mut |loan| {
            if Self::loan_may_lend(&loan) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_break()
    }

    fn loan_may_lend(loan: &Loan<'e>) -> bool {
        match loan {
            Loan::Temporary(_) => true,
            Loan::Operand(operand, _) => {
                let operand = Self::new(operand);
                operand.borrowed_temporary().is_some() || operand.may_lend()
            }
        }
    }

    pub fn for_each_loan(self, scope: TemporaryScope, mut lend: impl FnMut(Loan<'e>)) {
        let _: ControlFlow<Infallible> = self.try_each_loan(scope, &mut |loan| {
            lend(loan);
            ControlFlow::Continue(())
        });
    }

    pub fn borrowed_temporary(self) -> Option<&'e TypedExpr> {
        match &self.expr.kind {
            TypedExprKind::Borrow(_, value) if Self::is_temporary(value) => Some(value),
            _ => None,
        }
    }

    fn try_each_loan<B>(
        self,
        scope: TemporaryScope,
        lend: &mut impl FnMut(Loan<'e>) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        match &self.expr.kind {
            TypedExprKind::Call(_, arguments) => arguments
                .iter()
                .try_for_each(|argument| lend(Self::argument_loan(argument))),
            TypedExprKind::Borrow(_, value) => lend(Loan::Operand(value, scope)),
            TypedExprKind::Variant(_, values, _) => values
                .iter()
                .try_for_each(|value| lend(Loan::Operand(value, scope))),
            TypedExprKind::Struct(structure) => self.struct_loans(structure, scope, lend),
            TypedExprKind::Try { value, .. } => {
                lend(Loan::Operand(value, TemporaryScope::Statement))
            }
            TypedExprKind::Control(control) => Self::control_loans(control, scope, lend),
            _ => ControlFlow::Continue(()),
        }
    }

    fn struct_loans<B>(
        self,
        structure: &'e TypedStructExpr,
        scope: TemporaryScope,
        lend: &mut impl FnMut(Loan<'e>) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        match structure {
            TypedStructExpr::New(_, fields) => fields
                .iter()
                .try_for_each(|(_, value)| lend(Loan::Operand(value, scope))),
            TypedStructExpr::Field(owner, _) => lend(self.owner_loan(owner, scope)),
            _ => ControlFlow::Continue(()),
        }
    }

    fn control_loans<B>(
        control: &'e TypedControl,
        scope: TemporaryScope,
        lend: &mut impl FnMut(Loan<'e>) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        match control {
            TypedControl::If(_, then, otherwise) => {
                lend(Loan::Operand(then, scope))?;
                lend(Loan::Operand(otherwise, scope))
            }
            TypedControl::Block(block) => block.tail().map_or(ControlFlow::Continue(()), |tail| {
                lend(Loan::Operand(tail, scope))
            }),
            TypedControl::Match(matched) => {
                lend(Loan::Operand(&matched.value, TemporaryScope::Statement))?;
                matched
                    .arms
                    .iter()
                    .filter_map(|arm| arm.body.tail())
                    .try_for_each(|tail| lend(Loan::Operand(tail, scope)))
            }
        }
    }

    fn argument_loan(argument: &'e TypedArgument) -> Loan<'e> {
        if Self::lends_temporary(argument) {
            Loan::Temporary(&argument.value)
        } else {
            Loan::Operand(&argument.value, TemporaryScope::Statement)
        }
    }

    fn owner_loan(self, owner: &'e TypedExpr, scope: TemporaryScope) -> Loan<'e> {
        if scope == TemporaryScope::Statement && self.lends_owner(owner) {
            Loan::Temporary(owner)
        } else {
            Loan::Operand(owner, TemporaryScope::Statement)
        }
    }

    fn lends_temporary(argument: &TypedArgument) -> bool {
        argument.borrow == Borrow::Shared
            && !argument.value.ty.is_reference()
            && Self::is_temporary(&argument.value)
    }

    fn lends_owner(self, owner: &'e TypedExpr) -> bool {
        let borrows_a_temporary = Self::new(owner).borrowed_temporary().is_some();
        self.expr.ty.is_reference() && !borrows_a_temporary && Self::is_temporary(owner)
    }

    fn is_temporary(value: &TypedExpr) -> bool {
        match &value.kind {
            TypedExprKind::Leaf(_) | TypedExprKind::Deref(_) => false,
            TypedExprKind::Borrow(_, inner) => Self::is_temporary(inner),
            _ => !value.is_place(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LendingExpression;
    use crate::emit::test_program::TestProgram;

    struct LendingFixture {
        program: TestProgram,
    }

    impl LendingFixture {
        const SOURCE: &str = "
            (defstruct label :text (ref str))
            (type first-letter (fn ((ref str)) (ref str)))
            (defn first-letter (text) (subs text 0 1))
            (type announced (fn ((ref str)) string))
            (defn announced (text) (str text))
            (defn lent-argument () (local t (first-letter (as-str (announced \"a\")))) t)
            (defn named-argument (text) (local t (first-letter text)) t)
            (defn scrutinee-borrow () (local t (match (ref (announced \"ab\")) (x (first-letter x)))) t)
            (defn written-borrow () (local t (ref (announced \"a\"))) t)
            (defn field-of-temporary () (local t (first-letter (:text (->label :text \"a\")))) t)
            (defn plain-number () (local n (+ 1 2)) n)
            (defn main ())";

        fn new() -> Self {
            Self {
                program: TestProgram::new(Self::SOURCE),
            }
        }

        fn may_lend(&self, function: &str) -> bool {
            LendingExpression::new(self.program.last_let_value(function)).may_lend()
        }

        fn borrows_a_temporary(&self, function: &str) -> bool {
            LendingExpression::new(self.program.last_let_value(function))
                .borrowed_temporary()
                .is_some()
        }
    }

    #[test]
    fn a_call_on_a_temporary_may_lend() {
        assert!(LendingFixture::new().may_lend("lent-argument"));
    }

    #[test]
    fn a_call_on_a_local_lends_nothing() {
        assert!(!LendingFixture::new().may_lend("named-argument"));
    }

    #[test]
    fn a_written_borrow_of_a_scrutinee_may_lend() {
        assert!(LendingFixture::new().may_lend("scrutinee-borrow"));
    }

    #[test]
    fn a_written_borrow_that_rust_extends_lends_nothing() {
        assert!(!LendingFixture::new().may_lend("written-borrow"));
    }

    #[test]
    fn a_field_of_a_temporary_may_lend() {
        assert!(LendingFixture::new().may_lend("field-of-temporary"));
    }

    #[test]
    fn arithmetic_lends_nothing() {
        assert!(!LendingFixture::new().may_lend("plain-number"));
    }

    #[test]
    fn names_the_temporary_a_written_borrow_takes() {
        assert!(LendingFixture::new().borrows_a_temporary("written-borrow"));
    }

    #[test]
    fn a_match_is_not_itself_a_borrowed_temporary() {
        assert!(!LendingFixture::new().borrows_a_temporary("scrutinee-borrow"));
    }
}
