use rustc_hash::FxHashMap;

use super::checking_facts::CheckingFacts;
use super::declarations::Declarations;
use crate::core::{BindingArena, BindingId};

#[derive(Clone, Copy)]
pub(super) struct WrittenTypeFacts<'l> {
    pub(super) bindings: &'l BindingArena,
    pub(super) declarations: &'l Declarations,
    pub(super) checking_facts: &'l CheckingFacts,
    pub(super) type_arities: &'l FxHashMap<BindingId, usize>,
}
