use std::path::PathBuf;

use super::example_question::ExampleQuestion;
use super::example_runs::ExampleRuns;
use super::linted_program::LintedProgram;
use super::linter::Linter;
use super::program_candidates::ProgramCandidates;
use super::program_position::ProgramPosition;
use super::program_run::ProgramRun;
use super::rule_failure::RuleFailure;
use crate::core::{CoreModule, ModuleId};
use crate::expand::Program;

pub struct UnlintedExamples<'r, Run: ProgramRun> {
    run: &'r mut Run,
    question: Option<Result<ExampleQuestion, RuleFailure>>,
}

impl<'r, Run: ProgramRun> UnlintedExamples<'r, Run> {
    pub fn new(run: &'r mut Run) -> Self {
        Self {
            run,
            question: None,
        }
    }

    pub fn mark<Declaration>(
        &mut self,
        candidates: &ProgramCandidates<Declaration>,
        programs: &mut [LintedProgram<'_>],
    ) {
        let held = self.held_files(programs);
        for (position, linted) in programs.iter_mut().enumerate() {
            if !candidates.tries(ProgramPosition::nth(position)) {
                continue;
            }
            let asked = self.unheld_modules(linted.program, &held);
            linted.holds_unlinted_example =
                !asked.is_empty() && self.holds_example(linted.program, &asked);
        }
    }

    fn holds_example(&mut self, program: &Program, asked: &[ModuleId]) -> bool {
        let run = &mut *self.run;
        let question = self.question.get_or_insert_with(|| run.example_question());
        match question {
            Ok(question) => Linter::new(program, self.run.sources())
                .holds_example(question, asked)
                .unwrap_or(true),
            Err(_) => true,
        }
    }

    fn held_files(&self, programs: &[LintedProgram<'_>]) -> Vec<PathBuf> {
        programs
            .iter()
            .filter(|linted| linted.examples == ExampleRuns::EveryModule)
            .flat_map(|linted| linted.program.project_modules())
            .filter_map(|module| self.file_of(module))
            .collect()
    }

    fn unheld_modules(&self, program: &Program, held: &[PathBuf]) -> Vec<ModuleId> {
        program
            .project_modules()
            .filter(|module| {
                self.file_of(module)
                    .is_none_or(|file| !held.contains(&file))
            })
            .map(|module| module.id)
            .collect()
    }

    fn file_of(&self, module: &CoreModule) -> Option<PathBuf> {
        let item = module.items.first()?;
        self.run.source_path(item.span.source())
    }
}
