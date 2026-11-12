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
use crate::judgement::Judgement;
use crate::knave_compiler::KnaveCompiler;
use crate::oracle::Oracle;
use crate::reducer::Reducer;
use crate::subject_program::SubjectProgram;

pub struct Harness {
    backends: Vec<Box<dyn Backend>>,
    oracles: Vec<Box<dyn Oracle>>,
    slots: JobSlots,
    jobs: usize,
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
            jobs: jobs.max(1),
        }
    }

    pub fn keeping_oracles(mut self, names: &[String]) -> Result<Self, String> {
        if let Some(unknown) = names.iter().find(|name| {
            !self
                .oracles
                .iter()
                .any(|oracle| oracle.name() == name.as_str())
        }) {
            return Err(format!(
                "no oracle is named `{unknown}`; the oracles are {}",
                self.oracle_names().join(", ")
            ));
        }
        if !names.is_empty() {
            self.oracles
                .retain(|oracle| names.iter().any(|name| name == oracle.name()));
        }
        Ok(self)
    }

    pub fn oracle_names(&self) -> Vec<&'static str> {
        self.oracles.iter().map(|oracle| oracle.name()).collect()
    }

    pub fn jobs(&self) -> usize {
        self.jobs
    }

    pub fn backend_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.backends.iter().map(|backend| backend.name())
    }

    pub fn judge(&self, program: SubjectProgram, limit: Duration) -> Judgement {
        let _slot = self.slots.take();
        let verdicts = BackendVerdicts::collect(program, &self.backend_list(), limit);
        let findings = self
            .oracles
            .iter()
            .filter_map(|oracle| oracle.judge(&verdicts))
            .collect();
        Judgement { verdicts, findings }
    }

    pub fn reduced(&self, finding: &Finding, reducer: &dyn Reducer) -> SubjectProgram {
        let _slot = self.slots.take();
        match self
            .oracles
            .iter()
            .find(|oracle| oracle.name() == finding.oracle)
        {
            Some(oracle) => reducer.shrink(
                finding.program.clone(),
                oracle.as_ref(),
                &self.backend_list(),
            ),
            None => finding.program.clone(),
        }
    }

    fn backend_list(&self) -> Vec<&dyn Backend> {
        self.backends.iter().map(Box::as_ref).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::Harness;
    use crate::knave_compiler::KnaveCompiler;

    struct HarnessFixture;

    impl HarnessFixture {
        const JOBS: usize = 2;

        fn harness() -> Harness {
            Harness::new(KnaveCompiler::at(PathBuf::from("knave")), Self::JOBS)
        }
    }

    #[test]
    fn keeps_every_oracle_when_none_is_named() {
        let harness = HarnessFixture::harness()
            .keeping_oracles(&[])
            .expect("no names");
        assert_eq!(
            harness.oracle_names(),
            ["CheckAcceptsRustcRejects", "FalseOwnershipError"]
        );
        assert_eq!(harness.jobs(), HarnessFixture::JOBS);
    }

    #[test]
    fn keeps_only_the_named_oracle() {
        let names = ["FalseOwnershipError".to_string()];
        let harness = HarnessFixture::harness()
            .keeping_oracles(&names)
            .expect("a known name");
        assert_eq!(harness.oracle_names(), ["FalseOwnershipError"]);
    }

    #[test]
    fn refuses_an_unknown_oracle_name() {
        let names = ["Telepathy".to_string()];
        let refusal = HarnessFixture::harness().keeping_oracles(&names).err();
        assert_eq!(
            refusal.as_deref(),
            Some(
                "no oracle is named `Telepathy`; the oracles are CheckAcceptsRustcRejects, FalseOwnershipError"
            )
        );
    }
}
