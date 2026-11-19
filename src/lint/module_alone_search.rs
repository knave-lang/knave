use super::declared_functions::DeclaredFunctions;
use super::library_declarations::LibraryDeclarations;
use super::needless_type_search::NeedlessTypeSearch;
use super::needless_type_trial::NeedlessTypeTrial;
use super::program_run::ProgramRun;
use crate::core::{BindingId, ModulePath};
use crate::expand::Program;

pub struct ModuleAloneSearch<'r, Run: ProgramRun> {
    run: &'r mut Run,
}

impl<'r, Run: ProgramRun> ModuleAloneSearch<'r, Run> {
    pub fn new(run: &'r mut Run) -> Self {
        Self { run }
    }

    pub fn removable_names(self, library: &LibraryDeclarations) -> Vec<String> {
        let Some(mut alone) = self.run.module_alone(&library.module) else {
            return Vec::new();
        };
        Self::check_every_definition_of(&mut alone, &library.module);
        let Ok(declared) = self.run.check(&alone) else {
            return Vec::new();
        };
        let tried: Vec<BindingId> = DeclaredFunctions::new(&alone)
            .named_in(&library.module, &library.names)
            .iter()
            .map(|declaration| declaration.function)
            .collect();
        let mut trial = NeedlessTypeTrial::new(self.run, &mut alone, declared);
        let removable = NeedlessTypeSearch::new(&mut trial)
            .jointly_removable(&tried)
            .removable;
        let names = DeclaredFunctions::new(&alone);
        removable
            .into_iter()
            .map(|function| names.name(function))
            .collect()
    }

    fn check_every_definition_of(alone: &mut Program, module: &ModulePath) {
        let defined: Vec<BindingId> = alone
            .modules
            .iter()
            .filter(|candidate| &candidate.path == module)
            .flat_map(|candidate| candidate.definitions().map(|(binding, _)| binding))
            .collect();
        for binding in defined {
            alone.check_also(binding);
        }
    }
}
