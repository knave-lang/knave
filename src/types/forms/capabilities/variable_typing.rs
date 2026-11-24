use super::{Bindings, GlobalReferences, LocalTypes, VariantArms};

pub(in crate::types) trait VariableTyping:
    Bindings + VariantArms + LocalTypes + GlobalReferences
{
}

impl<C> VariableTyping for C where C: Bindings + VariantArms + LocalTypes + GlobalReferences {}
