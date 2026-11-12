use std::io;

use crate::timed_command::{Completion, Finished};
use crate::verdict::Verdict;

pub struct ToolRun {
    tool: &'static str,
}

impl ToolRun {
    pub fn of(tool: &'static str) -> Self {
        Self { tool }
    }

    pub fn finished(&self, completion: io::Result<Completion>) -> Result<Finished, Verdict> {
        match completion {
            Ok(Completion::Finished(finished)) => Ok(finished),
            Ok(Completion::TimedOut) => Err(Verdict::Unsupported(format!(
                "{} ran past the time limit",
                self.tool
            ))),
            Err(error) => Err(Verdict::Unsupported(format!(
                "cannot run {}: {error}",
                self.tool
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::ToolRun;
    use crate::timed_command::Completion;
    use crate::verdict::Verdict;

    #[test]
    fn a_timeout_or_a_missing_tool_is_unsupported_and_names_the_tool() {
        let run = ToolRun::of("rustc");
        let timed_out = run.finished(Ok(Completion::TimedOut)).err();
        let missing = run
            .finished(Err(io::Error::from(io::ErrorKind::NotFound)))
            .err();
        assert!(
            matches!(timed_out, Some(Verdict::Unsupported(reason)) if reason.starts_with("rustc ran past"))
        );
        assert!(
            matches!(missing, Some(Verdict::Unsupported(reason)) if reason.starts_with("cannot run rustc"))
        );
    }
}
