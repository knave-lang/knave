use super::copy::CopyRules;
use super::typed::TypedBinder;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterCopy {
    Read,
    Dereference,
    Clone,
}

impl ParameterCopy {
    pub fn of(param: &TypedBinder, copy: &CopyRules) -> Self {
        if !copy.is_copy(param.ty.referent()) {
            ParameterCopy::Clone
        } else if param.annotated {
            ParameterCopy::Dereference
        } else {
            ParameterCopy::Read
        }
    }

    pub fn expression(self, param: &str) -> String {
        match self {
            ParameterCopy::Read => param.to_string(),
            ParameterCopy::Dereference => format!("(deref {param})"),
            ParameterCopy::Clone => format!("(clone {param})"),
        }
    }
}
