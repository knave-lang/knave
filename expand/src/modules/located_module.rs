use std::rc::Rc;

use super::module_source::ModuleSource;

pub struct LocatedModule {
    pub source: ModuleSource,
    pub owner: Option<Rc<str>>,
}
