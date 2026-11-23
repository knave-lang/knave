use crate::core::ModulePath;
use crate::language::Language;

pub struct ReservedModulePaths;

impl ReservedModulePaths {
    pub const PRELUDE: &str = "prelude";
    pub const KERNEL: &str = "kernel";
    pub const CODE: &str = "code";
    pub const PRELUDE_CODE: &str = "prelude/internal/code";

    pub fn is_reserved_library(name: &str) -> bool {
        [Self::PRELUDE, Self::KERNEL, Self::CODE].contains(&name)
            || Self::is_reserved(&ModulePath::new(name))
    }

    pub fn is_reserved(path: &ModulePath) -> bool {
        path.as_str() == Language::PROJECT_FILE_STEM
    }

    pub fn is_compiler_provided(path: &ModulePath) -> bool {
        [Self::KERNEL, Self::CODE, Self::PRELUDE_CODE].contains(&path.as_str())
    }
}
