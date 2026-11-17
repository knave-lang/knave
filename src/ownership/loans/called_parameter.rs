use crate::types::{TemplateTies, Type};

#[derive(Clone, Copy)]
pub struct CalledParameter<'t> {
    ty: Option<&'t Type>,
    stores_the_reference: bool,
}

impl<'t> CalledParameter<'t> {
    pub fn untied(ty: Option<&'t Type>) -> Self {
        Self {
            ty,
            stores_the_reference: false,
        }
    }

    pub fn at(ty: Option<&'t Type>, ties: Option<&TemplateTies>, position: usize) -> Self {
        Self {
            ty,
            stores_the_reference: ties.is_some_and(|ties| ties.stores_the_reference(position)),
        }
    }

    pub fn ty(&self) -> Option<&'t Type> {
        self.ty
    }

    pub fn stores_the_reference(&self) -> bool {
        self.stores_the_reference
    }
}
