use super::conflict_kind::ConflictKind;
use super::loan_conflict::LoanConflict;
use super::loan_holder::LoanHolder;
use crate::core::BindingArena;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct LoanConflictProblems<'c> {
    conflict: &'c LoanConflict<'c>,
    bindings: &'c BindingArena,
}

impl<'c> LoanConflictProblems<'c> {
    pub fn new(conflict: &'c LoanConflict<'c>, bindings: &'c BindingArena) -> Self {
        Self { conflict, bindings }
    }

    pub fn diagnostic(&self) -> Diagnostic {
        match self.conflict.access().access().conflict_kind() {
            ConflictKind::Change => self.changed_while_lent(),
            ConflictKind::Move => self.moved_while_lent(),
            ConflictKind::BorrowDuringMut => self.borrowed_while_lent_as_mut(),
        }
    }

    fn changed_while_lent(&self) -> Diagnostic {
        let (place, holder) = (self.place(), self.holder());
        self.labeled(
            Diagnostic::error(
                DiagnosticCode::CHANGED_WHILE_LENT,
                format!("`{place}` changes while {holder} still borrows it"),
                Label::new(self.access_span(), format!("`{place}` changes here")),
            ),
            "the change",
        )
        .with_help(self.help("changing"))
    }

    fn moved_while_lent(&self) -> Diagnostic {
        let (place, holder) = (self.place(), self.holder());
        self.labeled(
            Diagnostic::error(
                DiagnosticCode::MOVED_WHILE_LENT,
                format!("`{place}` moves while {holder} still borrows it"),
                Label::new(self.access_span(), format!("`{place}` moves here")),
            ),
            "the move",
        )
        .with_help(self.help("moving"))
    }

    fn borrowed_while_lent_as_mut(&self) -> Diagnostic {
        let (place, holder) = (self.place(), self.holder());
        self.labeled(
            Diagnostic::error(
                DiagnosticCode::CHANGED_WHILE_LENT,
                format!("`{place}` is borrowed while {holder} still borrows it as `mut`"),
                Label::new(self.access_span(), format!("`{place}` is borrowed here")),
            ),
            "this borrow",
        )
        .with_help(self.finishing_help("borrowing"))
    }

    fn help(&self, acting: &str) -> String {
        let finishing = self.finishing_help(acting);
        match self.conflict.holder() {
            LoanHolder::Value(_) => {
                format!(
                    "{finishing}, or copy what you need out of `{}` first",
                    self.lent()
                )
            }
            LoanHolder::Call | LoanHolder::Result => finishing,
        }
    }

    fn finishing_help(&self, acting: &str) -> String {
        let (place, lent) = (self.place(), self.lent());
        match self.conflict.holder() {
            LoanHolder::Value(_) => {
                format!("finish using {} before {acting} `{place}`", self.holder())
            }
            LoanHolder::Call => {
                format!("bind a `clone` of `{lent}` to a local before the call, and pass the local")
            }
            LoanHolder::Result => {
                format!("finish {acting} `{place}` before borrowing the result from `{lent}`")
            }
        }
    }

    fn labeled(&self, diagnostic: Diagnostic, conflicting: &str) -> Diagnostic {
        let lent = Label::new(
            self.conflict.lending().span(),
            format!("{} borrows `{}` here", self.holder(), self.lent()),
        );
        let diagnostic = diagnostic.with_label(lent);
        match self.conflict.later_use() {
            Some(span) => diagnostic.with_label(Label::new(
                span,
                format!("{} is used here, after {conflicting}", self.holder()),
            )),
            None => diagnostic,
        }
    }

    fn access_span(&self) -> Span {
        self.conflict.access().span()
    }

    fn place(&self) -> String {
        self.conflict.access().place().named(self.bindings)
    }

    fn lent(&self) -> String {
        self.conflict.lending().place().named(self.bindings)
    }

    fn holder(&self) -> String {
        self.conflict.holder().subject(self.bindings)
    }
}
