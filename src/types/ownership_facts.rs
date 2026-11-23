use rustc_hash::{FxHashMap, FxHashSet};

use super::parameter_modes::ParameterModes;
use crate::core::BindingId;

#[derive(Default)]
pub struct OwnershipFacts {
    pub parameter_modes: ParameterModes,
    pub stored_function_parameters: FxHashSet<BindingId>,
    pub escaping_lambdas: FxHashSet<BindingId>,
    pub borrowing_lambdas: FxHashMap<BindingId, FxHashSet<BindingId>>,
    pub mutable_function_bindings: FxHashSet<BindingId>,
}
