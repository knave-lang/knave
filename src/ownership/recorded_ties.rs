use crate::types::{TemplateBorrows, TemplateTies, TypedExpr, TypedProgram};

pub struct RecordedTies<'p> {
    templates: &'p TemplateBorrows,
}

impl<'p> RecordedTies<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            templates: &program.template_borrows,
        }
    }

    pub fn of(&self, callee: &TypedExpr) -> Option<TemplateTies> {
        self.templates
            .read_template(callee, None, |_, ties| ties.cloned())
    }
}
