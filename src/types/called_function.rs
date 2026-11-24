use super::typed::{TypedArgument, TypedLambda};
use crate::core::BindingId;

pub struct CalledFunction<'p, 'e> {
    pub function: BindingId,
    pub lambda: &'p TypedLambda,
    pub arguments: &'e [TypedArgument],
}
