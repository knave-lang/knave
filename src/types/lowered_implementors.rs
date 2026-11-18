use super::lowered_associated_value::LoweredAssociatedValue;
use super::trait_rows_reach::Implementor;
use crate::diagnostic::Diagnostic;

#[derive(Default)]
pub(super) struct LoweredImplementors {
    pub(super) implementors: Vec<(Implementor, Vec<LoweredAssociatedValue>)>,
    pub(super) failure: Option<Diagnostic>,
}
