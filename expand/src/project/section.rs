use super::dependency_section::DependencySection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Dependency(DependencySection),
    Lints,
    ExportedTasks,
}

impl Section {
    pub(super) const ALL: [Section; 5] = [
        Section::Dependency(DependencySection::Libraries),
        Section::Dependency(DependencySection::Dependencies),
        Section::Dependency(DependencySection::DevDependencies),
        Section::Lints,
        Section::ExportedTasks,
    ];
}
