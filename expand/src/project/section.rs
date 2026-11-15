use super::dependency_section::DependencySection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Dependency(DependencySection),
    Lints,
}

impl Section {
    pub(super) const ALL: [Section; 4] = [
        Section::Dependency(DependencySection::Libraries),
        Section::Dependency(DependencySection::Dependencies),
        Section::Dependency(DependencySection::DevDependencies),
        Section::Lints,
    ];
}
