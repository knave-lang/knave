use super::open_round_end::OpenRoundEnd;
use super::settlement_failure::SettlementFailure;
use super::settlement_verdicts::SettlementVerdicts;
use crate::diagnostic::Diagnostics;

pub(super) struct RoundFailure {
    reported: Diagnostics,
    open: OpenRoundEnd,
    trailing: Diagnostics,
}

impl RoundFailure {
    pub(super) fn new(reported: Diagnostics, open: OpenRoundEnd, trailing: Diagnostics) -> Self {
        Self {
            reported,
            open,
            trailing,
        }
    }

    pub(super) fn fails(reported: &Diagnostics, open: &OpenRoundEnd) -> bool {
        reported.has_errors() || !open.missing.is_empty() || !open.held.is_empty()
    }

    pub(super) fn alone(self) -> Diagnostics {
        self.ended(SettlementVerdicts::default())
    }

    pub(super) fn ended(self, verdicts: SettlementVerdicts) -> Diagnostics {
        let bounds = self.shown_bounds(&verdicts);
        let Self {
            mut reported,
            open,
            trailing,
        } = self;
        let explained = reported.has_errors() || !open.missing.is_empty();
        match verdicts.into_failure() {
            Some(SettlementFailure::Refused(refusals)) => reported.extend(refusals),
            _ if explained => reported.extend(open.missing),
            Some(SettlementFailure::Waiting(waiting)) => reported.extend(waiting),
            None => {}
        }
        reported.extend(bounds);
        reported.extend(trailing);
        reported
    }

    fn shown_bounds(&self, verdicts: &SettlementVerdicts) -> Diagnostics {
        let shown = (self.open.held.iter()).filter_map(|held| held.after(verdicts, &self.reported));
        Diagnostics::from_iter(shown).distinct()
    }
}
