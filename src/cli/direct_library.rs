use std::path::PathBuf;

use super::checkout_bounds::CheckoutBounds;
use super::located_project::ProjectSite;

use super::lender_arguments::LenderArguments;
use crate::project::LibraryDependency;

#[derive(Clone, Debug)]
pub struct DirectLibrary {
    name: String,
    root: PathBuf,
    exported_tasks: Vec<String>,
    fetched: bool,
}

impl DirectLibrary {
    pub fn of(library: &LibraryDependency, root: PathBuf, exported_tasks: Vec<String>) -> Self {
        Self {
            name: library.name.clone(),
            root,
            exported_tasks,
            fetched: library.source.is_fetched(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn site(&self) -> ProjectSite<'_> {
        ProjectSite::Library {
            root: &self.root,
            checkout: self.fetched.then(|| CheckoutBounds::of(&self.root)),
        }
    }

    pub fn lends_tasks(&self) -> bool {
        !self.exported_tasks.is_empty()
    }

    pub fn exports(&self, task: &str) -> bool {
        self.exported_tasks.iter().any(|exported| exported == task)
    }

    pub fn exported_tasks(&self) -> &[String] {
        &self.exported_tasks
    }

    pub fn arguments(&self) -> LenderArguments<'_> {
        if self.fetched {
            LenderArguments::fetched(&self.name, &self.exported_tasks)
        } else {
            LenderArguments::local(&self.name, &self.exported_tasks)
        }
    }
}
