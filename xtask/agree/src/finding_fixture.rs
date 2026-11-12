use std::collections::BTreeMap;
use std::path::PathBuf;

use knave_expand::diagnostic::DiagnosticCode;

use crate::backend_verdicts::BackendVerdicts;
use crate::citation::Citation;
use crate::finding::Finding;
use crate::seed::Seed;
use crate::subject_program::SubjectProgram;
use crate::verdict::{RejectingTool, Rejection};

pub struct FindingFixture;

impl FindingFixture {
    const MAIN: &str = "(defn main () (println 1))\n";

    pub fn citing(program: &str, codes: &[&str]) -> Finding {
        Self::citing_for("Oracle", program, codes)
    }

    pub fn citing_for(oracle: &'static str, program: &str, codes: &[&str]) -> Finding {
        Self::finding(oracle, Self::program(program, Self::MAIN), codes)
    }

    pub fn seeded(seed: Seed, codes: &[&str]) -> Finding {
        let program =
            SubjectProgram::generated(seed, PathBuf::from("main.knv"), Self::MAIN.to_string());
        Self::finding("Oracle", program, codes)
    }

    pub fn program(name: &str, text: &str) -> SubjectProgram {
        SubjectProgram::of_entry(
            name.to_string(),
            PathBuf::from("main.knv"),
            text.to_string(),
        )
    }

    fn finding(oracle: &'static str, program: SubjectProgram, codes: &[&str]) -> Finding {
        let verdicts = BackendVerdicts {
            program,
            verdicts: BTreeMap::new(),
        };
        let evidence = codes.iter().map(|code| Self::rejection(code)).collect();
        Finding::of(oracle, &verdicts, Citation::new("judged", evidence))
    }

    fn rejection(code: &str) -> Rejection {
        Rejection {
            tool: RejectingTool::Rustc,
            code: Some(DiagnosticCode::from_text(code.to_string())),
            file: PathBuf::from("main.rs"),
            span: 0..0,
            function: Some("main".to_string()),
            reached_from_main: true,
        }
    }
}
