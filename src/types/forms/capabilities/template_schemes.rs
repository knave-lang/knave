use std::rc::Rc;

use crate::core::RustTemplate;
use crate::diagnostic::Diagnostic;
use crate::types::ty::Scheme;

pub(in crate::types) trait TemplateSchemes {
    fn lower_template(&mut self, template: &RustTemplate) -> Result<Scheme, Diagnostic>;
    fn register_inline(&mut self, text: &Rc<str>, scheme: &Scheme);
}
