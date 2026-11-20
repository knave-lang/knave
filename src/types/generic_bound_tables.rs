use rustc_hash::FxHashMap;

use super::traits::TraitTable;
use super::ty::{GenericId, GenericParam};

#[derive(Clone, Copy)]
pub(super) struct GenericBoundTables<'a> {
    pub(super) traits: &'a TraitTable,
    pub(super) generics: &'a FxHashMap<GenericId, GenericParam>,
}
