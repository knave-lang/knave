use super::{BindingNames, GlobalReferences, LocalScope, VariantArms};

pub(in crate::types) trait VariableTyping:
    BindingNames + VariantArms + LocalScope + GlobalReferences
{
}

impl<C> VariableTyping for C where C: BindingNames + VariantArms + LocalScope + GlobalReferences {}
