use rustc_hash::FxHashSet;

use super::control_flow_graph::ControlFlowGraph;
use super::point_id::PointId;
use crate::core::BindingId;

pub struct LiveValues {
    live_before: Vec<FxHashSet<BindingId>>,
}

impl LiveValues {
    pub fn of(graph: &ControlFlowGraph) -> Self {
        let mut live = Self {
            live_before: graph.points().map(|_| FxHashSet::default()).collect(),
        };
        while live.grow(graph) {}
        live
    }

    pub fn before(&self, point: PointId) -> &FxHashSet<BindingId> {
        &self.live_before[point.index()]
    }

    fn grow(&mut self, graph: &ControlFlowGraph) -> bool {
        let mut grew = false;
        let mut live = FxHashSet::default();
        for (point, facts) in graph.points().rev() {
            live.clear();
            for successor in facts.successors() {
                live.extend(self.live_before[successor.index()].iter().copied());
            }
            facts.narrow_to_live_before(&mut live);
            let known = &mut self.live_before[point.index()];
            if live.len() != known.len() {
                grew = true;
                std::mem::swap(known, &mut live);
            }
        }
        grew
    }
}

#[cfg(test)]
mod tests {
    use super::LiveValues;
    use crate::core::BindingId;
    use crate::ownership::loans::control_flow_graph::ControlFlowGraph;
    use crate::ownership::loans::point_id::PointId;
    use crate::source::{SourceId, Span};

    #[test]
    fn a_value_is_live_from_its_definition_to_its_last_use_and_around_a_loop() {
        let value = BindingId::from_index(7);
        let mut graph = ControlFlowGraph::new();
        let defined = graph.join(&[PointId::ENTRY]);
        graph.point_mut(defined).define(value);
        let head = graph.join(&[defined]);
        let used = graph.join(&[head]);
        graph
            .point_mut(used)
            .using(value, Span::new(SourceId::new(0), 0..1));
        let back = graph.join(&[used]);
        graph.link(back, head);
        let after = graph.join(&[back]);
        let live = LiveValues::of(&graph);
        assert!(!live.before(defined).contains(&value));
        assert!(live.before(head).contains(&value));
        assert!(live.before(back).contains(&value));
        assert!(!live.before(after).contains(&value));
    }
}
