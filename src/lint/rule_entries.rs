use crate::core::BindingId;

#[derive(Clone, Copy)]
pub struct RuleEntries {
    pub check: BindingId,
    pub names: BindingId,
}
