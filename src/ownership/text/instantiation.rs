use super::access::Access;
use super::scheme_position::SchemePosition;
use super::scheme_positions::SchemePositions;
use crate::types::{GenericId, Type};

#[derive(Clone, Copy)]
pub struct Instantiation<'t> {
    declared: &'t Type,
    instantiated: &'t Type,
}

impl<'t> Instantiation<'t> {
    pub fn new(declared: &'t Type, instantiated: &'t Type) -> Self {
        Self {
            declared,
            instantiated,
        }
    }

    pub fn instantiated(&self) -> &'t Type {
        self.instantiated
    }

    pub fn positions(&self) -> Vec<SchemePosition> {
        SchemePositions::of(self.declared, self.instantiated)
    }

    pub fn whole_generic(&self) -> Option<(GenericId, Access)> {
        Self::whole_generic_of(self.declared)
    }

    fn whole_generic_of(declared: &Type) -> Option<(GenericId, Access)> {
        match declared {
            Type::Generic(generic) => Some((*generic, Access::Direct)),
            Type::Ref(_, inner) => {
                Self::whole_generic_of(inner).map(|(generic, _)| (generic, Access::Referenced))
            }
            _ => None,
        }
    }
}
