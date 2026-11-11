use std::rc::Rc;

use crate::reader::Form;
use crate::source::Span;

pub trait SourceQueries {
    fn form_location(&self, form: &Form) -> String;
    fn in_tasks_directory(&self, form: &Form) -> bool;
    fn introducing_macro(&self, form: &Form) -> Option<Rc<str>>;
    fn read_literal_form(
        &self,
        literal: Span,
        value: &str,
        character: usize,
    ) -> Option<(Form, usize)>;
}
