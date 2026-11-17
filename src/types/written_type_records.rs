use super::written_projection::WrittenProjection;
use crate::diagnostic::Diagnostics;

pub(super) struct WrittenTypeRecords<'l> {
    pub(super) projections: &'l mut Vec<WrittenProjection>,
    pub(super) diagnostics: &'l mut Diagnostics,
}
