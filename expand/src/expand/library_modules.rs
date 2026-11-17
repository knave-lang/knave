use std::rc::Rc;

use crate::core::ModulePath;

#[derive(Default)]
pub(super) struct LibraryModules {
    owners: Vec<(ModulePath, Rc<str>)>,
}

impl LibraryModules {
    pub fn remember(&mut self, path: &ModulePath, owner: Rc<str>) {
        self.owners.push((path.clone(), owner));
    }

    pub fn owner_of(&self, path: &ModulePath) -> Option<Rc<str>> {
        self.owners
            .iter()
            .find(|(loaded, _)| loaded == path)
            .map(|(_, owner)| Rc::clone(owner))
    }
}
