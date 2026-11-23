use super::live_loan::LiveLoan;
use super::loan_conflict::LoanConflict;
use super::loan_names::LoanNames;
use super::place_access::PlaceAccess;
use super::point_graph::PointGraph;
use super::point_id::PointId;

pub struct AccessFacts<'b> {
    point: PointId,
    access: &'b PlaceAccess,
    live: Vec<LiveLoan<'b>>,
}

impl<'b> AccessFacts<'b> {
    pub fn new(point: PointId, access: &'b PlaceAccess, live: Vec<LiveLoan<'b>>) -> Self {
        Self {
            point,
            access,
            live,
        }
    }

    pub fn conflicts(&self, graph: &PointGraph) -> Vec<LoanConflict<'b>> {
        self.live
            .iter()
            .filter_map(|loan| loan.conflict_with(self.access, self.point, graph))
            .collect()
    }

    pub fn written(&self, names: &LoanNames<'_>) -> String {
        let live: Vec<String> = self.live.iter().map(|loan| loan.written(names)).collect();
        let live = if live.is_empty() {
            "no live loan".to_string()
        } else {
            live.join("; ")
        };
        format!("{}: {live}", self.access.written(names))
    }
}
