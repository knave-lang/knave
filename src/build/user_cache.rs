use std::env;
use std::path::PathBuf;

use crate::language::Language;

pub struct UserCache;

impl UserCache {
    const CACHE_HOME: &str = "XDG_CACHE_HOME";
    const HOME: &str = "HOME";
    const CACHE_UNDER_HOME: &str = ".cache";

    pub fn directory() -> Option<PathBuf> {
        Self::absolute_path(Self::CACHE_HOME)
            .or_else(|| {
                Self::absolute_path(Self::HOME).map(|home| home.join(Self::CACHE_UNDER_HOME))
            })
            .map(|cache| cache.join(Language::NAME))
    }

    fn absolute_path(name: &str) -> Option<PathBuf> {
        Self::variable_path(name).filter(|path| path.is_absolute())
    }

    pub fn variable_path(name: &str) -> Option<PathBuf> {
        env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }
}
