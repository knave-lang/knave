use std::collections::BTreeMap;
use std::path::PathBuf;

use knave_expand::diagnostic::DiagnosticCode;

use crate::backend_verdicts::BackendVerdicts;
use crate::subject_program::SubjectProgram;
use crate::verdict::{RejectingTool, Rejection, Verdict};

pub struct OracleFixture;

impl OracleFixture {
    const MAIN_ONLY: &str = "(defn main () (println 1))";

    pub fn verdicts(verdicts: [(&'static str, Verdict); 2]) -> BackendVerdicts {
        BackendVerdicts {
            program: SubjectProgram::of_entry(
                "probe".to_string(),
                PathBuf::from("probe.knv"),
                Self::MAIN_ONLY.to_string(),
            ),
            verdicts: BTreeMap::from(verdicts),
        }
    }

    pub fn rejected_by_knave(code: &str) -> Verdict {
        Self::rejected(RejectingTool::Knave, code)
    }

    pub fn rejected_by_rustc(code: &str) -> Verdict {
        Self::rejected(RejectingTool::Rustc, code)
    }

    pub fn rejected_by_knave_without_a_code() -> Verdict {
        Self::rejected_with(RejectingTool::Knave, None)
    }

    pub fn rejected_by_knave_in_an_unreached_function(code: &str) -> Verdict {
        Verdict::Rejected(vec![Rejection {
            function: Some("unused".to_string()),
            reached_from_main: false,
            ..Self::rejection(RejectingTool::Knave, Self::code(code))
        }])
    }

    fn rejected(tool: RejectingTool, code: &str) -> Verdict {
        Self::rejected_with(tool, Self::code(code))
    }

    fn code(code: &str) -> Option<DiagnosticCode> {
        Some(DiagnosticCode::from_text(code.to_string()))
    }

    fn rejected_with(tool: RejectingTool, code: Option<DiagnosticCode>) -> Verdict {
        Verdict::Rejected(vec![Self::rejection(tool, code)])
    }

    fn rejection(tool: RejectingTool, code: Option<DiagnosticCode>) -> Rejection {
        Rejection {
            tool,
            code,
            file: PathBuf::from("probe.knv"),
            span: 0..0,
            function: Some("main".to_string()),
            reached_from_main: true,
        }
    }
}
