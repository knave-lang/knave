use super::bound_origin_problems::BoundOriginProblems;
use super::bound_origins::BoundOrigins;
use super::field_bound_problems::FieldBoundProblems;
use super::inferred_at::InferredAt;
use super::missing_read::MissingRead;
use super::settlement_failure::SettlementFailure;
use super::settlement_verdicts::SettlementVerdicts;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::Span;

pub(super) struct HeldBound {
    problems: FieldBoundProblems,
    open_read: String,
    origins: BoundOrigins,
}

impl HeldBound {
    pub(super) fn new(
        problems: FieldBoundProblems,
        open_read: String,
        origins: BoundOrigins,
    ) -> Self {
        Self {
            problems,
            open_read,
            origins,
        }
    }

    pub(super) fn after(
        &self,
        verdicts: &SettlementVerdicts,
        reported: &Diagnostics,
    ) -> Option<Diagnostic> {
        let reads = self.reads();
        if verdicts.refuses_any(&reads) {
            return None;
        }
        match verdicts.agreed_read(&reads) {
            Some(read) => Some(self.with(MissingRead::Known(read.to_string()))),
            None => (!reported.has_errors() && !Self::waits(verdicts))
                .then(|| self.with(MissingRead::Open(self.open_read.clone()))),
        }
    }

    fn waits(verdicts: &SettlementVerdicts) -> bool {
        matches!(
            verdicts.first_failure(),
            Some(SettlementFailure::Waiting(_))
        )
    }

    fn reads(&self) -> Vec<Span> {
        InferredAt::of(&self.origins).spans().collect()
    }

    fn with(&self, read: MissingRead) -> Diagnostic {
        BoundOriginProblems::annotate(self.problems.missing(&read), &self.origins)
    }
}
