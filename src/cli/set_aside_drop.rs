use super::name_records::NameRecords;
use crate::diagnostic::Diagnostic;

pub trait SetAsideDrop {
    fn warning(&self, setter: &str) -> Diagnostic;

    fn record(&self, records: &mut NameRecords, setter: &str);
}
