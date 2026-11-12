use std::collections::BTreeMap;
use std::path::PathBuf;

use knave_expand::diagnostic::DiagnosticCode;

use crate::backend_verdicts::BackendVerdicts;
use crate::citation::Citation;
use crate::finding::Finding;
use crate::subject_program::SubjectProgram;
use crate::verdict::{RejectingTool, Rejection};

pub struct FindingFixture;

impl FindingFixture {
    pub fn citing(program: &str, codes: &[&str]) -> Finding {
        let verdicts = BackendVerdicts {
            program: Self::program(program, "(defn main () (println 1))\n"),
            verdicts: BTreeMap::new(),
        };
        let evidence = codes.iter().map(|code| Self::rejection(code)).collect();
        Finding::of("Oracle", &verdicts, Citation::new("judged", evidence))
    }

    pub fn program(name: &str, text: &str) -> SubjectProgram {
        SubjectProgram::of_entry(
            name.to_string(),
            PathBuf::from("main.knv"),
            text.to_string(),
        )
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
