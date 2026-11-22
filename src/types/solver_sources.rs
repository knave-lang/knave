use rustc_hash::FxHashMap;

use super::declarations::Declarations;
use super::ty::{GenericId, GenericParam};

#[derive(Clone, Copy)]
pub(super) struct SolverSources<'a> {
    pub(super) declarations: &'a Declarations,
    pub(super) generics: &'a FxHashMap<GenericId, GenericParam>,
}
