#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Libraries,
    Dependencies,
    DevDependencies,
}

impl Section {
    pub(super) const ALL: [Section; 3] = [
        Section::Libraries,
        Section::Dependencies,
        Section::DevDependencies,
    ];
    const LIBRARY_KEYS: &[&str] = &["path", "git", "rev"];
    const CRATE_KEYS: &[&str] = &["features", "path"];

    pub(super) fn keys(self) -> &'static [&'static str] {
        match self {
            Section::Libraries => Self::LIBRARY_KEYS,
            Section::Dependencies | Section::DevDependencies => Self::CRATE_KEYS,
        }
    }
}
