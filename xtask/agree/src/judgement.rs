use crate::backend_verdicts::BackendVerdicts;
use crate::finding::Finding;

pub struct Judgement {
    pub verdicts: BackendVerdicts,
    pub findings: Vec<Finding>,
}
