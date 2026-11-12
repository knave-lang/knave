use super::error_location::ModuleLoader;
use crate::check::ProgramChecker;
use crate::diagnostic::Diagnostics;
use crate::expand::{BoundName, Program};
use crate::ownership::EnforcedOwnership;

pub struct SourcesSurvey {
    diagnostics: Diagnostics,
    names: Vec<Vec<BoundName>>,
    loaders: Vec<ModuleLoader>,
}

impl SourcesSurvey {
    pub fn new(
        program: Result<Program, Diagnostics>,
        names: Vec<Vec<BoundName>>,
        loaders: Vec<ModuleLoader>,
    ) -> Self {
        Self {
            diagnostics: program
                .map_or_else(|errors| errors, |program| Self::check_quietly(&program)),
            names,
            loaders,
        }
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    pub fn loaders(&self) -> &[ModuleLoader] {
        &self.loaders
    }

    pub fn into_names_and_loaders(self) -> (Vec<Vec<BoundName>>, Vec<ModuleLoader>) {
        (self.names, self.loaders)
    }

    fn check_quietly(program: &Program) -> Diagnostics {
        match ProgramChecker::check::<EnforcedOwnership>(program) {
            Ok(typed) => typed.warnings,
            Err(diagnostics) => diagnostics,
        }
    }
}
