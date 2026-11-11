use std::borrow::Cow;

use super::fragment::{Code, Fragment};
use crate::types::TypedExpr;

pub struct LiftedTemporary {
    pub name: String,
    pub value: TypedExpr,
}

pub struct LiftedValue<'v, T: Clone> {
    pub temporaries: Vec<LiftedTemporary>,
    pub value: Cow<'v, T>,
}

impl<'v, T: Clone> LiftedValue<'v, T> {
    pub fn unchanged(value: &'v T) -> Self {
        Self {
            temporaries: Vec::new(),
            value: Cow::Borrowed(value),
        }
    }

    pub fn statements(&self, mut emit: impl FnMut(&TypedExpr) -> Code) -> Vec<Fragment> {
        let mut statements = Vec::new();
        for temporary in &self.temporaries {
            statements.push(Fragment::Text(format!("let {} = ", temporary.name)));
            statements.extend(emit(&temporary.value).fragments);
            statements.push(Fragment::Text(";".to_string()));
        }
        statements
    }
}
