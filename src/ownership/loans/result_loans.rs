use std::collections::BTreeSet;

use rustc_hash::FxHashSet;

use super::control_flow_graph::ControlFlowGraph;
use super::loan_id::LoanId;
use super::loan_table::LoanTable;
use super::point_id::PointId;

pub struct ResultLoans {
    reached: Vec<(LoanId, FxHashSet<PointId>)>,
}

impl ResultLoans {
    pub fn of(graph: &ControlFlowGraph, loans: &LoanTable, returned: &BTreeSet<LoanId>) -> Self {
        Self {
            reached: returned
                .iter()
                .map(|loan| (*loan, graph.reachable_from(loans.loan(*loan).made_at())))
                .collect(),
        }
    }

    pub fn live_at(&self, point: PointId) -> impl Iterator<Item = LoanId> + '_ {
        self.reached
            .iter()
            .filter(move |(_, points)| points.contains(&point))
            .map(|(loan, _)| *loan)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::ResultLoans;
    use crate::core::{BindingId, ReferenceKind};
    use crate::ownership::loans::control_flow_graph::ControlFlowGraph;
    use crate::ownership::loans::lending::Lending;
    use crate::ownership::loans::loan_id::LoanId;
    use crate::ownership::loans::loan_kind::LoanKind;
    use crate::ownership::loans::loan_place::LoanPlace;
    use crate::ownership::loans::loan_table::LoanTable;
    use crate::ownership::loans::point_id::PointId;
    use crate::source::{SourceId, Span};
    use crate::types::{Type, TypedExpr, TypedExprKind, TypedLeaf};

    struct ResultFixture;

    impl ResultFixture {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn lent_local(loans: &mut LoanTable, point: PointId) -> LoanId {
            let local = TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Local(BindingId::from_index(1))),
                ty: Type::unit(),
                span: Self::span(),
            };
            let place = LoanPlace::of(&local).expect("a local is a place");
            let kind = <dyn LoanKind>::of(ReferenceKind::Shared);
            loans.record(Lending::new(place, kind, Self::span()), point)
        }
    }

    #[test]
    fn a_returned_loan_is_live_at_every_point_reachable_from_it() {
        let mut graph = ControlFlowGraph::new();
        let mut loans = LoanTable::default();
        let branch = graph.join(&[PointId::ENTRY]);
        let other = graph.join(&[PointId::ENTRY]);
        let loan = ResultFixture::lent_local(&mut loans, branch);
        let exit = graph.join(&[branch, other]);
        let result = ResultLoans::of(&graph, &loans, &BTreeSet::from([loan]));
        assert_eq!(result.live_at(exit).collect::<Vec<_>>(), [loan]);
        assert_eq!(result.live_at(other).count(), 0);
        assert_eq!(result.live_at(PointId::ENTRY).count(), 0);
    }
}
