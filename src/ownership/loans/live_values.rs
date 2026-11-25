use rustc_hash::{FxHashMap, FxHashSet};

use super::point_graph::PointGraph;
use super::point_id::PointId;
use crate::core::BindingId;

pub struct LiveValues {
    live_before: Vec<FxHashSet<BindingId>>,
}

impl LiveValues {
    pub fn of(graph: &PointGraph) -> Self {
        Self::holding(graph, &FxHashMap::default())
    }

    pub fn holding(graph: &PointGraph, flowing: &FxHashMap<PointId, Vec<BindingId>>) -> Self {
        let mut live = Self {
            live_before: graph.points().map(|_| FxHashSet::default()).collect(),
        };
        while live.grow(graph, flowing) {}
        live
    }

    pub fn before(&self, point: PointId) -> &FxHashSet<BindingId> {
        &self.live_before[point.index()]
    }

    fn grow(&mut self, graph: &PointGraph, flowing: &FxHashMap<PointId, Vec<BindingId>>) -> bool {
        let mut grew = false;
        let mut live = FxHashSet::default();
        for (point, facts) in graph.points().rev() {
            live.clear();
            for successor in facts.successors() {
                live.extend(self.live_before[successor.index()].iter().copied());
            }
            facts.narrow_to_live_before(&mut live);
            live.extend(flowing.get(&point).into_iter().flatten().copied());
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
        let mut graph = ControlFlowGraph::default();
        let defined = graph.join(&[PointId::ENTRY]);
        graph.define(defined, value);
        let head = graph.join(&[defined]);
        let used = graph.join(&[head]);
        graph.using(used, value, Span::new(SourceId::new(0), 0..1));
        let back = graph.join(&[used]);
        graph.link(back, head);
        let after = graph.join(&[back]);
        let live = LiveValues::of(&graph.finished());
        assert!(!live.before(defined).contains(&value));
        assert!(live.before(head).contains(&value));
        assert!(live.before(back).contains(&value));
        assert!(!live.before(after).contains(&value));
    }

    #[test]
    fn a_value_flowing_into_a_later_point_is_live_until_that_point() {
        let value = BindingId::from_index(7);
        let mut graph = ControlFlowGraph::default();
        let defined = graph.join(&[PointId::ENTRY]);
        graph.define(defined, value);
        let read = graph.join(&[defined]);
        let consumed = graph.join(&[read]);
        let after = graph.join(&[consumed]);
        let flowing = [(consumed, vec![value])].into_iter().collect();
        let live = LiveValues::holding(&graph.finished(), &flowing);
        assert!(live.before(read).contains(&value));
        assert!(live.before(consumed).contains(&value));
        assert!(!live.before(after).contains(&value));
    }
}
