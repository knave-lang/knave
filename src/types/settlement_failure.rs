use crate::diagnostic::Diagnostics;

pub(super) enum SettlementFailure {
    Refused(Diagnostics),
    Waiting(Diagnostics),
}
