use crate::core::BindingId;

pub struct Removal<Kept> {
    pub removable: Vec<BindingId>,
    pub kept: Option<Kept>,
}

impl<Kept> Default for Removal<Kept> {
    fn default() -> Self {
        Self {
            removable: Vec::new(),
            kept: None,
        }
    }
}
