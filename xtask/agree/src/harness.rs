use std::time::Duration;

use crate::backend::Backend;
use crate::backend_verdicts::BackendVerdicts;
use crate::check_accepts_rustc_rejects::CheckAcceptsRustcRejects;
use crate::checker::Checker;
use crate::checker_without_ownership::CheckerWithoutOwnership;
use crate::compiled_debug::CompiledDebug;
use crate::false_ownership_error::FalseOwnershipError;
use crate::finding::Finding;
use crate::job_slots::JobSlots;
use crate::knave_compiler::KnaveCompiler;
use crate::oracle::Oracle;
use crate::subject_program::SubjectProgram;

pub struct Harness {
    backends: Vec<Box<dyn Backend>>,
    oracles: Vec<Box<dyn Oracle>>,
    slots: JobSlots,
}

impl Harness {
    pub fn new(compiler: KnaveCompiler, jobs: usize) -> Self {
        Self {
            backends: vec![
                Box::new(Checker::new(compiler.clone())),
                Box::new(CheckerWithoutOwnership::new(compiler.clone())),
                Box::new(CompiledDebug::new(compiler)),
            ],
            oracles: vec![
                Box::new(CheckAcceptsRustcRejects),
                Box::new(FalseOwnershipError),
            ],
            slots: JobSlots::new(jobs),
        }
    }

    pub fn backend_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.backends.iter().map(|backend| backend.name())
    }

    pub fn verdicts(&self, program: SubjectProgram, limit: Duration) -> BackendVerdicts {
        let _slot = self.slots.take();
        let backends: Vec<&dyn Backend> = self.backends.iter().map(Box::as_ref).collect();
        BackendVerdicts::collect(program, &backends, limit)
    }

    pub fn findings(&self, verdicts: &BackendVerdicts) -> Vec<Finding> {
        self.oracles
            .iter()
            .filter_map(|oracle| oracle.judge(verdicts))
            .collect()
    }
}
