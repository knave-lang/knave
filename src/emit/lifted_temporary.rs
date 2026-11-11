use super::fragment::{Code, Fragment};
use super::let_statement::LetStatement;
use crate::types::TypedExpr;

pub struct LiftedTemporary {
    name: String,
    value: Option<TypedExpr>,
}

impl LiftedTemporary {
    pub fn bound(name: String, value: TypedExpr) -> Self {
        Self {
            name,
            value: Some(value),
        }
    }

    pub fn declared(name: String) -> Self {
        Self { name, value: None }
    }

    pub fn statement(&self, emit: impl FnOnce(&TypedExpr) -> Code) -> Vec<Fragment> {
        let statement = match &self.value {
            Some(value) => LetStatement::new(self.name.as_str(), emit(value)),
            None => LetStatement::declared(self.name.as_str()),
        };
        statement.fragments()
    }

    #[cfg(test)]
    pub fn value(&self) -> Option<&TypedExpr> {
        self.value.as_ref()
    }
}
