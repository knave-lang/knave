use crate::backend_verdicts::BackendVerdicts;
use crate::finding::Finding;

pub trait Oracle: Send + Sync {
    fn name(&self) -> &'static str;
    fn judge(&self, verdicts: &BackendVerdicts) -> Option<Finding>;
}
