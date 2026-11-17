use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::ty::{GenericId, GenericParam};
use super::unifier::LiteralKind;

pub(super) struct DeclaredGenerics {
    pub(super) params: FxHashMap<GenericId, GenericParam>,
    pub(super) associated_types: FxHashMap<GenericId, Rc<str>>,
    pub(super) literals: FxHashMap<GenericId, LiteralKind>,
}
