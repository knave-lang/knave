use super::module_load_site::ModuleLoadSite;
use crate::modules::RealPath;

pub(super) struct TextModuleLoad {
    pub(super) site: ModuleLoadSite,
    pub(super) file: Option<RealPath>,
}
