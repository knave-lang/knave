use std::borrow::Cow;

use super::expression_holder::ExpressionHolder;
use super::fragment::Fragment;
use super::lifted_temporary::LiftedTemporary;

pub struct LiftedValue<'v, T: ExpressionHolder> {
    temporaries: Vec<LiftedTemporary>,
    value: Cow<'v, T>,
}

impl<'v, T: ExpressionHolder> LiftedValue<'v, T> {
    pub fn new(temporaries: Vec<LiftedTemporary>, value: Cow<'v, T>) -> Self {
        Self { temporaries, value }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn statements(&self) -> Vec<Fragment> {
        self.temporaries
            .iter()
            .flat_map(LiftedTemporary::statement)
            .collect()
    }
}
