use std::rc::Rc;

use super::direct_library::DirectLibrary;

#[derive(Clone, Debug, Default)]
pub struct LendingLibraries {
    libraries: Vec<DirectLibrary>,
}

impl LendingLibraries {
    pub fn new(libraries: Vec<DirectLibrary>) -> Self {
        Self { libraries }
    }

    pub fn named(&self, name: &str) -> Option<&DirectLibrary> {
        self.libraries.iter().find(|library| library.name() == name)
    }

    pub fn names(&self) -> Vec<Rc<str>> {
        self.libraries
            .iter()
            .map(|library| Rc::from(library.name()))
            .collect()
    }

    pub fn lenders(&self) -> Vec<&DirectLibrary> {
        self.sorted(DirectLibrary::lends_tasks)
    }

    pub fn exporters_of(&self, task: &str) -> Vec<&DirectLibrary> {
        self.sorted(|library| library.exports(task))
    }

    fn sorted(&self, keep: impl Fn(&DirectLibrary) -> bool) -> Vec<&DirectLibrary> {
        let mut kept: Vec<&DirectLibrary> = self
            .libraries
            .iter()
            .filter(|library| keep(library))
            .collect();
        kept.sort_by(|left, right| left.name().cmp(right.name()));
        kept
    }

    pub fn rows(&self, own: &[Rc<str>]) -> Vec<Rc<str>> {
        let lent = self.libraries.iter().flat_map(|library| {
            library
                .exported_tasks()
                .iter()
                .map(|task| Rc::from(self.row_name(library, task, own)))
        });
        own.iter().cloned().chain(lent).collect()
    }

    pub fn listing_arguments(&self, library: &DirectLibrary, own: &[Rc<str>]) -> String {
        library
            .arguments()
            .listing(|task| self.qualifies(task, own))
    }

    fn row_name(&self, library: &DirectLibrary, task: &str, own: &[Rc<str>]) -> String {
        if self.qualifies(task, own) {
            format!("{}/{task}", library.name())
        } else {
            task.to_string()
        }
    }

    fn qualifies(&self, task: &str, own: &[Rc<str>]) -> bool {
        let shadowed = own.iter().any(|name| name.as_ref() == task);
        shadowed || self.exporters_of(task).len() > 1
    }
}
