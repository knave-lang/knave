use std::rc::Rc;

use crate::core::ModulePath;
use crate::source::Span;

#[derive(Clone)]
pub(super) struct ModuleLoadSite {
    pub(super) path: ModulePath,
    pub(super) span: Span,
    pub(super) owner: Option<Rc<str>>,
    pub(super) importer: Option<Rc<str>>,
}
