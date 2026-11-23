use std::rc::Rc;

use super::callee_visit::CalleeVisit;
use super::stored_flows::DefinitionStores;
use crate::core::BindingId;
use crate::types::{Type, TypedProgram};

pub(super) trait RegionRules {
    fn program(&self) -> &TypedProgram;
    fn carries_borrow(&self, ty: &Type) -> bool;
    fn callee_visit(&self, callee: BindingId) -> CalleeVisit;
    fn stores_of(&self, definition: BindingId) -> Rc<DefinitionStores>;
}
