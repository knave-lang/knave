#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencySection {
    Libraries,
    Dependencies,
    DevDependencies,
}

impl DependencySection {
    const LIBRARY_KEYS: &[&str] = &["path", "git", "rev"];
    const CRATE_KEYS: &[&str] = &["features", "path"];

    pub(super) fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Libraries => Self::LIBRARY_KEYS,
            Self::Dependencies | Self::DevDependencies => Self::CRATE_KEYS,
        }
    }
}
