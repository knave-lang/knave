use super::notes_too_large::NotesTooLarge;
use crate::reader::{Form, Symbol};

pub trait ModuleNotes {
    fn add_module_note(&mut self, key: &str, note: Form) -> Result<(), NotesTooLarge>;
    fn module_notes(&self, context: &Symbol, key: &str) -> Vec<Form>;
}
