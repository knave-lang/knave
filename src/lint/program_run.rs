use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::types::TypedProgram;

pub trait ProgramRun {
    fn check(&mut self, program: &Program) -> Result<TypedProgram, Diagnostics>;

    fn emit(&self, typed: &TypedProgram) -> Option<String>;

    fn module_alone(&mut self, module: &ModulePath) -> Option<Program>;
}
