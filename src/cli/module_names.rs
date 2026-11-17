use std::path::Path;

use crate::core::ModulePath;

pub struct ModuleNames;

impl ModuleNames {
    pub fn of_source(relative: &Path) -> Option<String> {
        ModulePath::of_file(relative).map(|path| path.as_str().to_string())
    }

    pub fn of_folder(relative: &Path) -> Option<String> {
        ModulePath::of_folder(relative).map(|path| path.as_str().to_string())
    }
}
