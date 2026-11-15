use super::recur_check::RecurCheck;
use super::try_check::TryCheck;
use crate::core::{Block, Expr};
use crate::diagnostic::Diagnostic;

pub struct TopLevelCheck;

impl TopLevelCheck {
    pub fn block(&self, block: &Block) -> Result<(), Diagnostic> {
        RecurCheck::outside().block(block)?;
        TryCheck::outside().block(block)
    }

    pub fn value(&self, expr: &Expr) -> Result<(), Diagnostic> {
        RecurCheck::outside().value(expr)?;
        TryCheck::outside().value(expr)
    }
}
