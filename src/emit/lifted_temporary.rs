use super::fragment::{Code, Fragment};
use super::let_statement::LetStatement;
use crate::types::TypedExpr;

pub struct LiftedTemporary {
    name: String,
    value: TypedExpr,
}

impl LiftedTemporary {
    pub fn new(name: String, value: TypedExpr) -> Self {
        Self { name, value }
    }

    pub fn statement(&self, emit: impl FnOnce(&TypedExpr) -> Code) -> Vec<Fragment> {
        LetStatement::new(self.name.as_str(), emit(&self.value)).fragments()
    }

    #[cfg(test)]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[cfg(test)]
    pub fn value(&self) -> &TypedExpr {
        &self.value
    }
}
