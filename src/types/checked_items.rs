use rustc_hash::FxHashMap;

use super::typed::{ClosureType, TypedDefinition};
use crate::core::{BindingId, CoreModule};

#[derive(Clone, Copy)]
pub(super) struct CheckedItems<'c> {
    pub(super) modules: &'c [CoreModule],
    pub(super) definitions: &'c FxHashMap<BindingId, TypedDefinition>,
    pub(super) closures: &'c [ClosureType],
}
