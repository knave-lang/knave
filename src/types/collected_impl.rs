use super::{Implementor, TypedTraitImpl};

pub(super) struct CollectedImpl {
    pub(super) implementor: Implementor,
    pub(super) definition: TypedTraitImpl,
}

impl CollectedImpl {
    pub(super) fn new(implementor: Implementor, definition: TypedTraitImpl) -> Self {
        Self {
            implementor,
            definition,
        }
    }
}
