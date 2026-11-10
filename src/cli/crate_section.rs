use crate::language::Language;
use crate::project::{CrateDependency, Project, Section};

#[derive(Clone, Copy)]
pub enum CrateSection {
    Dependencies,
    DevDependencies,
}

impl CrateSection {
    pub const DEV_FLAG_NAME: &'static str = "dev";
    pub const ALL: [Self; 2] = [Self::Dependencies, Self::DevDependencies];

    pub fn of(dev: bool) -> Self {
        if dev {
            Self::DevDependencies
        } else {
            Self::Dependencies
        }
    }

    pub fn section(self) -> Section {
        match self {
            Self::Dependencies => Section::Dependencies,
            Self::DevDependencies => Section::DevDependencies,
        }
    }

    pub fn name(self) -> &'static str {
        self.section().name()
    }

    pub fn listed(self, project: &Project) -> &[CrateDependency] {
        match self {
            Self::Dependencies => &project.dependencies,
            Self::DevDependencies => &project.dev_dependencies,
        }
    }

    pub fn command(self, command: &str) -> String {
        match self {
            Self::Dependencies => format!("{} {command}", Language::NAME),
            Self::DevDependencies => {
                format!("{} {command} --{}", Language::NAME, Self::DEV_FLAG_NAME)
            }
        }
    }
}
