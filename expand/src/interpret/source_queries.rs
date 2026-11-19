use std::rc::Rc;

use crate::modules::ProjectFolder;
use crate::reader::Form;
use crate::source::Span;

pub trait SourceQueries {
    fn form_location(&self, form: &Form) -> String;
    fn project_folders(&self, form: &Form) -> Vec<ProjectFolder>;
    fn introducing_macro(&self, form: &Form) -> Option<Rc<str>>;
    fn read_literal_form(
        &self,
        literal: Span,
        value: &str,
        character: usize,
    ) -> Option<(Form, usize)>;
}
