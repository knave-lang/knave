use rustc_hash::FxHashMap;

use super::ty::{GenericId, GenericParam};
use crate::core::BindingId;

#[derive(Default)]
pub(super) struct LoweredBinders {
    pub(super) lifetimes: Vec<BindingId>,
    pub(super) params: Vec<GenericParam>,
    pub(super) scope: FxHashMap<BindingId, GenericId>,
}
