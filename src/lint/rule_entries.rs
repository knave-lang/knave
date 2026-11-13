use crate::core::BindingId;

#[derive(Clone, Copy)]
pub struct RuleEntries {
    pub check: BindingId,
    pub check_module: BindingId,
    pub kept: BindingId,
    pub names: BindingId,
}
