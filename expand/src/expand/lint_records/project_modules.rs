use rustc_hash::FxHashSet;

use crate::core::{CoreModule, ModuleId};
use crate::modules::ModuleOrigin;

pub(in crate::expand) struct ProjectModules(FxHashSet<ModuleId>);

impl ProjectModules {
    pub(in crate::expand) fn of(modules: &[CoreModule]) -> Self {
        let project = modules.iter().filter(|module| {
            ModuleOrigin::new(&module.path, module.library.as_deref()).is_project()
        });
        Self(project.map(|module| module.id).collect())
    }

    pub(in crate::expand) fn contains(&self, module: ModuleId) -> bool {
        self.0.contains(&module)
    }
}
