use std::rc::Rc;

use super::read_back_table::ReadBackTable;
use crate::reader::Form;

pub struct ReadBackDefinition {
    pub name: Rc<str>,
    pub form: Form,
    pub table: ReadBackTable,
}
