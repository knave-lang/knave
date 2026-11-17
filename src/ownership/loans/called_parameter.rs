use crate::types::Type;

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

    pub fn storing(ty: Option<&'t Type>) -> Self {
        Self {
            ty,
            stores_the_reference: true,
        }
    }

    pub fn ty(&self) -> Option<&'t Type> {
        self.ty
    }

    pub fn stores_the_reference(&self) -> bool {
        self.stores_the_reference
    }
}
