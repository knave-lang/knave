use crate::finding::Finding;
use crate::run_tally::RunTally;

#[derive(Default)]
pub struct RunResults {
    pub findings: Vec<Finding>,
    pub tally: RunTally,
}
