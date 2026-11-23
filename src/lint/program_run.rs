use std::path::PathBuf;

use super::example_question::ExampleQuestion;
use super::program_position::ProgramPosition;
use super::rule_failure::RuleFailure;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::source::{SourceId, SourceMap};
use crate::types::TypedProgram;

pub trait ProgramRun {
    fn check(&mut self, program: &Program) -> Result<TypedProgram, Diagnostics>;

    fn emit(&self, typed: &TypedProgram) -> Option<String>;

    fn module_alone(&mut self, beside: ProgramPosition, module: &ModulePath) -> Option<Program>;

    fn source_path(&self, source: SourceId) -> Option<PathBuf>;

    fn example_question(&mut self) -> Result<ExampleQuestion, RuleFailure>;

    fn sources(&self) -> &SourceMap;
}
