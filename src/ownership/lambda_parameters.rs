use rustc_hash::FxHashSet;

use crate::core::BindingId;
use crate::types::TypedLambda;

#[derive(Default)]
pub struct LambdaParameters {
    all: FxHashSet<BindingId>,
    unwritten: FxHashSet<BindingId>,
}

impl LambdaParameters {
    pub fn of(lambda: &TypedLambda) -> Self {
        let all = lambda.params.iter().map(|param| param.binding).collect();
        let unwritten = lambda
            .params
            .iter()
            .filter(|param| !param.annotated)
            .map(|param| param.binding)
            .collect();
        Self { all, unwritten }
    }

    pub fn contains(&self, binding: BindingId) -> bool {
        self.all.contains(&binding)
    }

    pub fn is_unwritten(&self, binding: BindingId) -> bool {
        self.unwritten.contains(&binding)
    }
}
