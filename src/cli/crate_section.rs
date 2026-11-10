use crate::language::Language;
use crate::project::{CrateDependency, Project, Section};

#[derive(Clone, Copy)]
pub enum CrateSection {
    Dependencies,
    DevDependencies,
}

impl CrateSection {
    const DEV_FLAG: &'static str = " --dev";

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

    pub fn listed(self, project: &Project) -> &[CrateDependency] {
        match self {
            Self::Dependencies => &project.dependencies,
            Self::DevDependencies => &project.dev_dependencies,
        }
    }

    pub fn command(self, command: &str) -> String {
        let flag = match self {
            Self::Dependencies => "",
            Self::DevDependencies => Self::DEV_FLAG,
        };
        format!("{} {command}{flag}", Language::NAME)
    }
}
