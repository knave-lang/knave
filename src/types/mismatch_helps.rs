mod second_row_problems;

use std::rc::Rc;

use super::ty::Type;
use crate::diagnostic::{Diagnostic, FallbackHelp};
use crate::source::Span;
use second_row_problems::SecondRowProblems;

#[derive(Default)]
pub struct MismatchHelps {
    rows: Vec<MismatchHelpRow>,
}

pub(super) struct MismatchHelpRow {
    pub(super) expected: Type,
    pub(super) found: Type,
    pub(super) text: Rc<str>,
    pub(super) span: Span,
}

impl MismatchHelps {
    pub(super) fn add(&mut self, row: MismatchHelpRow) -> Result<(), Diagnostic> {
        if let Some(first) = self.row_for(&row.expected, &row.found) {
            return Err(SecondRowProblems::second_row(row.span, first.span));
        }
        self.rows.push(row);
        Ok(())
    }

    pub(super) fn help(&self, expected: &Type, found: &Type, value: Span) -> Option<FallbackHelp> {
        let row = self.row_for(expected, found)?;
        Some(FallbackHelp {
            message: row.text.to_string(),
            form: value,
        })
    }

    fn row_for(&self, expected: &Type, found: &Type) -> Option<&MismatchHelpRow> {
        self.rows
            .iter()
            .find(|row| row.expected == *expected && row.found == *found)
    }
}
