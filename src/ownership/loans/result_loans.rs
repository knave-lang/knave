use std::collections::BTreeSet;

use rustc_hash::FxHashSet;

use super::body_graph::BodyGraph;
use super::loan_id::LoanId;
use super::point_id::PointId;

pub struct ResultLoans {
    reached: Vec<(LoanId, FxHashSet<PointId>)>,
}

impl ResultLoans {
    pub fn of(graph: &BodyGraph, returned: &BTreeSet<LoanId>) -> Self {
        Self {
            reached: returned
                .iter()
                .map(|loan| (*loan, Self::reachable(graph, graph.loan(*loan).made_at())))
                .collect(),
        }
    }

    pub fn live_at(&self, point: PointId) -> impl Iterator<Item = LoanId> + '_ {
        self.reached
            .iter()
            .filter(move |(_, points)| points.contains(&point))
            .map(|(loan, _)| *loan)
    }

    fn reachable(graph: &BodyGraph, start: PointId) -> FxHashSet<PointId> {
        let successors: Vec<Vec<PointId>> = graph
            .points()
            .map(|(_, facts)| facts.successors().to_vec())
            .collect();
        let mut reached = FxHashSet::default();
        let mut pending = vec![start];
        while let Some(point) = pending.pop() {
            if reached.insert(point) {
                pending.extend(successors[point.index()].iter().copied());
            }
        }
        reached
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::ResultLoans;
    use crate::core::{BindingId, ReferenceKind};
    use crate::ownership::loans::body_graph::BodyGraph;
    use crate::ownership::loans::lending::Lending;
    use crate::ownership::loans::loan_place::LoanPlace;
    use crate::ownership::loans::point_id::PointId;
    use crate::source::{SourceId, Span};
    use crate::types::{Type, TypedExpr, TypedExprKind, TypedLeaf};

    #[test]
    fn a_returned_loan_is_live_at_every_point_reachable_from_it() {
        let mut graph = BodyGraph::new();
        let branch = graph.join(&[PointId::ENTRY]);
        let other = graph.join(&[PointId::ENTRY]);
        let span = Span::new(SourceId::new(0), 0..0);
        let local = TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Local(BindingId::from_index(1))),
            ty: Type::unit(),
            span,
        };
        let place = LoanPlace::of(&local).expect("a local is a place");
        let loan = graph.lend(Lending::new(place, ReferenceKind::Shared, span), branch);
        let exit = graph.join(&[branch, other]);
        let result = ResultLoans::of(&graph, &BTreeSet::from([loan]));
        assert_eq!(result.live_at(exit).collect::<Vec<_>>(), [loan]);
        assert_eq!(result.live_at(other).count(), 0);
        assert_eq!(result.live_at(PointId::ENTRY).count(), 0);
    }
}
