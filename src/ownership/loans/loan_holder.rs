use super::loan_names::LoanNames;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub enum LoanHolder {
    Value(BindingId),
    Call,
    Result,
}

impl LoanHolder {
    pub fn written(self, names: &LoanNames<'_>) -> String {
        match self {
            Self::Value(binding) => format!("held by `{}`", names.name(binding)),
            Self::Call => "passed to the call".to_string(),
            Self::Result => "returned".to_string(),
        }
    }
}
