use std::fmt;
use std::ops::Range;
use std::path::PathBuf;

use knave_expand::diagnostic::{CodeStage, DiagnosticCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Accepted,
    Rejected(Vec<Rejection>),
    Ran(RunOutcome),
    Unsupported(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectingTool {
    Knave,
    Rustc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub tool: RejectingTool,
    pub code: Option<DiagnosticCode>,
    pub file: PathBuf,
    pub span: Range<usize>,
    pub function: Option<String>,
    pub reached_from_main: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CitedPlace {
    code: Option<String>,
    function: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit: ExitKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitKind {
    Code(i32),
    Panic(String),
    StackOverflow,
    Signal(i32),
    TimedOut,
}

impl Verdict {
    pub fn outcome_name(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected(_) => "rejected",
            Self::Ran(outcome) if outcome.exit == ExitKind::TimedOut => "timed out",
            Self::Ran(_) => "ran",
            Self::Unsupported(_) => "unsupported",
        }
    }

    pub fn rejections(&self) -> &[Rejection] {
        match self {
            Self::Rejected(rejections) => rejections,
            _ => &[],
        }
    }
}

impl Rejection {
    const UNCODED: &'static str = "uncoded";

    pub fn is_ownership(&self) -> bool {
        self.tool == RejectingTool::Knave
            && self
                .code
                .as_ref()
                .is_some_and(|code| CodeStage::OWNERSHIP.contains(code))
    }

    pub fn place(&self) -> CitedPlace {
        CitedPlace {
            code: self.code.as_ref().map(|code| code.as_str().to_string()),
            function: self.function.clone(),
        }
    }

    pub fn is_rustc(&self) -> bool {
        self.tool == RejectingTool::Rustc
    }

    pub(crate) fn code_text(&self) -> &str {
        self.code
            .as_ref()
            .map_or(Self::UNCODED, DiagnosticCode::as_str)
    }
}

impl fmt::Display for Rejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.function {
            Some(function) => write!(formatter, "{} in `{function}`", self.code_text()),
            None => write!(formatter, "{} outside a function", self.code_text()),
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accepted => write!(formatter, "accepted"),
            Self::Rejected(rejections) => {
                let listed: Vec<String> = rejections.iter().map(Rejection::to_string).collect();
                write!(formatter, "rejected: {}", listed.join(", "))
            }
            Self::Ran(outcome) => write!(formatter, "ran: {outcome}"),
            Self::Unsupported(reason) => write!(formatter, "unsupported: {reason}"),
        }
    }
}

impl fmt::Display for RunOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?}, stdout {:?}, stderr {:?}",
            self.exit,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ExitKind, RunOutcome, Verdict};

    struct VerdictFixture;

    impl VerdictFixture {
        fn ran(exit: ExitKind) -> Verdict {
            Verdict::Ran(RunOutcome {
                stdout: Vec::new(),
                stderr: Vec::new(),
                exit,
            })
        }
    }

    #[test]
    fn names_each_outcome_and_tells_a_timeout_from_a_run() {
        let names: Vec<&str> = [
            Verdict::Accepted,
            Verdict::Rejected(Vec::new()),
            VerdictFixture::ran(ExitKind::TimedOut),
            VerdictFixture::ran(ExitKind::Code(0)),
            Verdict::Unsupported("crate".to_string()),
        ]
        .iter()
        .map(Verdict::outcome_name)
        .collect();
        assert_eq!(
            names,
            ["accepted", "rejected", "timed out", "ran", "unsupported"]
        );
    }
}
