use std::path::Path;

use crate::core::ModulePath;

pub(super) trait LibraryNaming {
    fn name(&self) -> &str;
    fn module_path_of(&self, inner: &Path) -> Option<ModulePath>;
}
