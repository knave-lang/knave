use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use knave_agree::{FixtureList, Harness, KnaveCompiler, ProgramSource, SubjectProgram};

pub struct AgreementReport {
    harness: Harness,
}

impl AgreementReport {
    const LIMIT: Duration = Duration::from_secs(120);
    const JOBS: usize = 4;

    pub fn new() -> Self {
        Self {
            harness: Harness::new(
                KnaveCompiler::at(PathBuf::from(env!("CARGO_BIN_EXE_knave"))),
                Self::JOBS,
                Self::LIMIT,
            ),
        }
    }

    pub fn of(&self, path: &Path) -> String {
        let program = FixtureList::new(vec![path.to_path_buf()])
            .programs()
            .next()
            .expect("the fixture list holds the fixture");
        let judgement = self.harness.judge(program);
        let verdicts = &judgement.verdicts;
        let mut report = String::new();
        for backend in self.harness.backend_names() {
            let verdict = verdicts.of(backend).expect("each backend gave a verdict");
            writeln!(report, "{backend}: {verdict}").expect("a string takes text");
        }
        let outside = Self::outside_the_claim(&verdicts.program);
        for finding in &judgement.findings {
            writeln!(
                report,
                "finding {}{outside}: {}",
                finding.oracle,
                finding.summary()
            )
            .expect("a string takes text");
        }
        report
    }

    fn outside_the_claim(program: &SubjectProgram) -> &'static str {
        if program.forms.uses_rust_templates() {
            " (outside the claim: the program writes a `rust` template)"
        } else {
            ""
        }
    }
}
