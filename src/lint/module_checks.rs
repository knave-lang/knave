use crate::core::BindingId;

#[derive(Default)]
pub struct ModuleChecks {
    pub refused: Vec<BindingId>,
    pub checked_alone: Vec<BindingId>,
}
