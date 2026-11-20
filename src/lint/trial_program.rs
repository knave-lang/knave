use std::mem;

use super::program_run::ProgramRun;
use crate::core::{BindingId, CoreModule, ItemKind};
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::types::TypedProgram;

pub struct TrialProgram<'p> {
    program: &'p mut Program,
    written: Vec<CoreModule>,
    definitions: Vec<BindingId>,
}

impl<'p> TrialProgram<'p> {
    pub fn new(program: &'p mut Program) -> Self {
        let written = program.modules.clone();
        let definitions = program
            .project_modules()
            .flat_map(CoreModule::definitions)
            .map(|(binding, _)| binding)
            .collect();
        Self {
            program,
            written,
            definitions,
        }
    }

    pub fn written(&self) -> &[CoreModule] {
        &self.written
    }

    pub fn definitions(&self) -> &[BindingId] {
        &self.definitions
    }

    pub fn checked_with(
        &mut self,
        modules: Vec<CoreModule>,
        run: &mut impl ProgramRun,
    ) -> Result<TypedProgram, Diagnostics> {
        let written = mem::replace(&mut self.program.modules, modules);
        let typed = run.check(self.program);
        self.program.modules = written;
        typed
    }

    pub fn holding(&self, errors: &Diagnostics, removed: &[BindingId]) -> Vec<BindingId> {
        self.written
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::Def(function, _) if removed.contains(function) => {
                    Some((*function, item.span))
                }
                _ => None,
            })
            .filter(|(_, span)| {
                errors
                    .into_iter()
                    .flat_map(|error| error.labels())
                    .any(|label| span.contains(label.span()))
            })
            .map(|(function, _)| function)
            .collect()
    }
}
