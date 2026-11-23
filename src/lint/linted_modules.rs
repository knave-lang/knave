use crate::core::CoreModule;
use crate::expand::Program;
#[cfg(test)]
use crate::modules::ModuleSources;

pub struct LintedModules<'p>(Vec<&'p CoreModule>);

impl<'p> LintedModules<'p> {
    pub fn of_project(program: &'p Program) -> Self {
        Self(program.project_modules().collect())
    }

    #[cfg(test)]
    pub fn of_prelude(program: &'p Program) -> Self {
        let prelude = program
            .modules
            .iter()
            .filter(|module| ModuleSources::is_prelude(&module.path));
        Self(prelude.collect())
    }

    pub fn iter(&self) -> impl Iterator<Item = &'p CoreModule> + '_ {
        self.0.iter().copied()
    }
}
