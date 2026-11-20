use super::conflict_parts::ConflictParts;
use super::conflict_wording::ConflictWording;
use super::loan_holder::LoanHolder;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};

pub struct LoanConflictProblems {
    parts: ConflictParts,
}

impl ConflictWording for LoanConflictProblems {
    fn changed_while_lent(&self) -> Diagnostic {
        let (place, holder) = (&self.parts.place, self.holder());
        self.labeled(
            Diagnostic::error(
                DiagnosticCode::CONFLICTS_WITH_LOAN,
                format!("`{place}` changes while {holder} still borrows it"),
                Label::new(self.parts.access_span, format!("`{place}` changes here")),
            ),
            "the change",
        )
        .with_help(self.help("changing"))
    }

    fn moved_while_lent(&self) -> Diagnostic {
        let (place, holder) = (&self.parts.place, self.holder());
        self.labeled(
            Diagnostic::error(
                DiagnosticCode::MOVED_WHILE_LENT,
                format!("`{place}` moves while {holder} still borrows it"),
                Label::new(self.parts.access_span, format!("`{place}` moves here")),
            ),
            "the move",
        )
        .with_help(self.help("moving"))
    }

    fn borrowed_while_lent_as_mut(&self) -> Diagnostic {
        let (place, holder) = (&self.parts.place, self.holder());
        self.labeled(
            Diagnostic::error(
                DiagnosticCode::CONFLICTS_WITH_LOAN,
                format!("`{place}` is borrowed while {holder} still borrows it as `mut`"),
                Label::new(
                    self.parts.access_span,
                    format!("`{place}` is borrowed here"),
                ),
            ),
            "this borrow",
        )
        .with_help(self.finishing_help("borrowing"))
    }
}

impl LoanConflictProblems {
    pub fn new(parts: ConflictParts) -> Self {
        Self { parts }
    }

    fn holder(&self) -> String {
        match (&self.parts.holder, &self.parts.holder_name) {
            (LoanHolder::Value(_), Some(name)) => format!("`{name}`"),
            (LoanHolder::Call, _) => "the call".to_string(),
            _ => "the result".to_string(),
        }
    }

    fn help(&self, acting: &str) -> String {
        let finishing = self.finishing_help(acting);
        match self.parts.holder {
            LoanHolder::Value(_) => format!(
                "{finishing}, or copy what you need out of `{}` first",
                self.parts.lent_place
            ),
            LoanHolder::Call | LoanHolder::Result => finishing,
        }
    }

    fn finishing_help(&self, acting: &str) -> String {
        let (place, lent) = (&self.parts.place, &self.parts.lent_place);
        match self.parts.holder {
            LoanHolder::Value(_) => {
                format!("finish using {} before {acting} `{place}`", self.holder())
            }
            LoanHolder::Call => {
                format!("bind a `clone` of `{lent}` to a local before the call, and pass the local")
            }
            LoanHolder::Result => format!(
                "the result keeps this borrow of `{lent}` on every path, so `{place}` cannot change while it lives; return a fresh borrow made after the change instead"
            ),
        }
    }

    fn labeled(&self, diagnostic: Diagnostic, conflicting: &str) -> Diagnostic {
        let lent = Label::new(
            self.parts.lent_span,
            format!("{} borrows `{}` here", self.holder(), self.parts.lent_place),
        );
        let diagnostic = diagnostic.with_label(lent);
        match self.parts.later_use {
            Some(span) => diagnostic.with_label(Label::new(
                span,
                format!("{} is used here, after {conflicting}", self.holder()),
            )),
            None => diagnostic,
        }
    }
}
