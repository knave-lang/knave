use crate::core::{BindingId, ModuleId};

pub struct ProgramEntry {
    pub module: ModuleId,
    pub function: Option<BindingId>,
}
