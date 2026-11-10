use std::fmt;

use super::copy::CopyRules;
use super::typed::TypedBinder;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParameterCopy<'n> {
    name: &'n str,
    form: CopyForm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CopyForm {
    Read,
    Dereference,
    Clone,
}

impl<'n> ParameterCopy<'n> {
    pub fn of(name: &'n str, param: &TypedBinder, copy: &CopyRules) -> Self {
        let form = if !copy.is_copy(param.ty.referent()) {
            CopyForm::Clone
        } else if param.annotated {
            CopyForm::Dereference
        } else {
            CopyForm::Read
        };
        Self { name, form }
    }

    pub fn name(&self) -> &'n str {
        self.name
    }
}

impl fmt::Display for ParameterCopy<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name;
        match self.form {
            CopyForm::Read => write!(formatter, "{name}"),
            CopyForm::Dereference => write!(formatter, "(deref {name})"),
            CopyForm::Clone => write!(formatter, "(clone {name})"),
        }
    }
}
