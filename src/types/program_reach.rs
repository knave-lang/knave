use std::collections::BTreeSet;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{BindingId, ModuleId};

#[derive(Default)]
pub struct ProgramReach {
    pub main: Option<BindingId>,
    pub reached: Vec<BindingId>,
    pub type_modules: FxHashMap<BindingId, ModuleId>,
    pub used_from_other_modules: FxHashSet<BindingId>,
    pub used_rust_types: BTreeSet<BindingId>,
}
