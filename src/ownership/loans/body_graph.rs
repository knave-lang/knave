use super::lending::Lending;
use super::loan::Loan;
use super::loan_id::LoanId;
use super::point_id::PointId;
use super::program_point::{PointFacts, ProgramPoint};

pub struct BodyGraph {
    points: Vec<ProgramPoint>,
    loans: Vec<Loan>,
}

impl BodyGraph {
    pub fn new() -> Self {
        Self {
            points: vec![ProgramPoint::default()],
            loans: Vec::new(),
        }
    }

    pub fn join(&mut self, before: &[PointId]) -> PointId {
        let after = PointId::new(self.points.len());
        self.points.push(ProgramPoint::default());
        for point in before {
            self.link(*point, after);
        }
        after
    }

    pub fn link(&mut self, before: PointId, after: PointId) {
        self.points[before.index()].precede(after);
    }

    pub fn point_mut(&mut self, point: PointId) -> &mut ProgramPoint {
        &mut self.points[point.index()]
    }

    pub fn lend(&mut self, lending: Lending, point: PointId) -> LoanId {
        self.point_mut(point).access(lending.access());
        let loan = LoanId::new(self.loans.len());
        self.loans.push(Loan::new(lending, point));
        loan
    }

    pub fn loan(&self, loan: LoanId) -> &Loan {
        &self.loans[loan.index()]
    }

    pub fn points(&self) -> impl DoubleEndedIterator<Item = (PointId, PointFacts<'_>)> + '_ {
        self.points
            .iter()
            .enumerate()
            .map(|(index, point)| (PointId::new(index), point.facts()))
    }
}

#[cfg(test)]
mod tests {
    use super::BodyGraph;
    use crate::core::ReferenceKind;
    use crate::ownership::lambda_source::LambdaSource;
    use crate::ownership::loans::lending::Lending;
    use crate::ownership::loans::loan_place::LoanPlace;
    use crate::ownership::loans::point_id::PointId;

    #[test]
    fn a_join_follows_each_point_before_it_and_a_loan_is_made_where_it_is_lent() {
        let source = LambdaSource::new(
            "(defn main ()\n  (local v (vec 1))\n  (local w v)\n  (println (len w)))",
        );
        let moved = source.bound("w");
        let mut graph = BodyGraph::new();
        let left = graph.join(&[PointId::ENTRY]);
        let right = graph.join(&[PointId::ENTRY]);
        let joined = graph.join(&[left, right]);
        let place = LoanPlace::of(moved).expect("a local is a place");
        let loan = graph.lend(
            Lending::new(place, ReferenceKind::Shared, moved.span),
            joined,
        );
        assert_eq!(graph.loan(loan).made_at(), joined);
        let facts: Vec<(Vec<PointId>, usize)> = graph
            .points()
            .map(|(_, facts)| (facts.successors().to_vec(), facts.accesses().len()))
            .collect();
        assert_eq!(
            facts,
            [
                (vec![left, right], 0),
                (vec![joined], 0),
                (vec![joined], 0),
                (vec![], 1)
            ]
        );
    }
}
