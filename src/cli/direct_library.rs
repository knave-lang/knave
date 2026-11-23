use std::path::PathBuf;

use super::checkout_bounds::CheckoutBounds;
use super::walk_start::WalkStart;

use super::lender_arguments::LenderArguments;
use crate::project::LibraryDependency;

#[derive(Clone, Debug)]
pub struct DirectLibrary {
    name: String,
    root: PathBuf,
    exported_tasks: Vec<String>,
    checkout: Option<CheckoutBounds>,
}

impl DirectLibrary {
    pub fn of(library: &LibraryDependency, root: PathBuf, exported_tasks: Vec<String>) -> Self {
        Self {
            name: library.name.clone(),
            checkout: CheckoutBounds::of_library(library, &root),
            root,
            exported_tasks,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn walk_start(&self) -> WalkStart<'_> {
        WalkStart {
            root: &self.root,
            checkout: self.checkout.clone(),
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
        if self.checkout.is_some() {
            LenderArguments::fetched(&self.name, &self.exported_tasks)
        } else {
            LenderArguments::local(&self.name, &self.exported_tasks)
        }
    }
}
