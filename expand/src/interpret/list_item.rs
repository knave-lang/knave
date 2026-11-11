use crate::reader::{Form, FormShape};

pub trait ListItem {
    fn list_size(&self) -> usize;
}

impl ListItem for Form {
    fn list_size(&self) -> usize {
        FormShape::from(self).stored_form_count()
    }
}
