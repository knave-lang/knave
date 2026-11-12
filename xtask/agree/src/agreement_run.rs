use std::sync::Mutex;
use std::thread;

use crate::harness::Harness;
use crate::run_results::RunResults;
use crate::subject_program::SubjectProgram;

pub struct AgreementRun<'a> {
    harness: &'a Harness,
}

impl<'a> AgreementRun<'a> {
    pub fn new(harness: &'a Harness) -> Self {
        Self { harness }
    }

    pub fn run(&self, programs: impl Iterator<Item = SubjectProgram>) -> RunResults {
        let queue = Mutex::new(programs.collect::<Vec<_>>().into_iter());
        let results = Mutex::new(RunResults::default());
        thread::scope(|scope| {
            for _ in 0..self.harness.jobs() {
                scope.spawn(|| self.drain(&queue, &results));
            }
        });
        results
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn drain(
        &self,
        queue: &Mutex<std::vec::IntoIter<SubjectProgram>>,
        results: &Mutex<RunResults>,
    ) {
        while let Some(program) = Self::next(queue) {
            let judgement = self.harness.judge(program);
            let mut results = results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            results.tally.count(&judgement.verdicts);
            results.findings.extend(judgement.findings);
        }
    }

    fn next(queue: &Mutex<std::vec::IntoIter<SubjectProgram>>) -> Option<SubjectProgram> {
        queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .next()
    }
}
