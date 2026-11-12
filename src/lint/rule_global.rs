use std::rc::Rc;

use crate::core::{Builtin, Expr};

pub enum RuleGlobal<'r> {
    Builtin(Builtin),
    Definition(&'r Rc<Expr>),
}
