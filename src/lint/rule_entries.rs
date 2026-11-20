use crate::core::BindingId;

#[derive(Clone, Copy)]
pub struct RuleEntries {
    pub check: BindingId,
    pub check_module: BindingId,
    pub kept: BindingId,
    pub examples: BindingId,
    pub names: BindingId,
}
