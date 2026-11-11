use std::borrow::Cow;

use super::emitted_value::EmittedValue;
use super::fragment::{Code, Fragment};
use super::lifted_temporary::LiftedTemporary;
use crate::types::TypedExpr;

pub struct LiftedValue<'v, T: EmittedValue> {
    temporaries: Vec<LiftedTemporary>,
    value: Cow<'v, T>,
}

impl<'v, T: EmittedValue> LiftedValue<'v, T> {
    pub fn new(temporaries: Vec<LiftedTemporary>, value: Cow<'v, T>) -> Self {
        Self { temporaries, value }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn statements(&self, mut emit: impl FnMut(&TypedExpr) -> Code) -> Vec<Fragment> {
        self.temporaries
            .iter()
            .flat_map(|temporary| temporary.statement(&mut emit))
            .collect()
    }

    #[cfg(test)]
    pub fn temporaries(&self) -> &[LiftedTemporary] {
        &self.temporaries
    }
}
