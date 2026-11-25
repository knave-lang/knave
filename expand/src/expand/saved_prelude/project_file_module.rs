use rustc_hash::FxHashSet;

use super::lone_prelude_module::LonePreludeModule;
use crate::core::ModulePath;
use crate::modules::EmbeddedModules;
use crate::project::{LevelKeys, ProjectRoles, ProjectVocabulary};
use crate::reader::Form;

pub struct ProjectFileModule {
    macros: FxHashSet<String>,
    roles: Vec<Form>,
    levels: Vec<Form>,
}

impl ProjectFileModule {
    pub fn expanded() -> Self {
        let path = ModulePath::new(EmbeddedModules::PROJECT_SHAPES);
        Self::of(&LonePreludeModule::expanded(&path))
    }

    pub fn vocabulary(&self) -> ProjectVocabulary {
        LevelKeys::read(&self.levels)
            .and_then(|levels| ProjectRoles::new(&self.macros).read(&self.roles, levels))
            .unwrap_or_else(|problem| {
                panic!("prelude/project-file states its project roles wrongly: {problem:?}")
            })
    }

    fn of(module: &LonePreludeModule) -> Self {
        Self {
            macros: module.macros(),
            roles: module.notes(ProjectRoles::NOTE),
            levels: module.notes(LevelKeys::NOTE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectFileModule;

    #[test]
    fn the_macros_are_the_ones_the_module_defines() {
        let macros = ProjectFileModule::expanded().macros;
        assert!(macros.contains("defproject"));
        assert!(!macros.contains("header-shape"));
    }
}
