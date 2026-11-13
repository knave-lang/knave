use rustc_hash::FxHashSet;

use super::body_graph::BodyGraph;
use super::point_id::PointId;
use crate::core::BindingId;

pub struct LiveValues {
    live_before: Vec<FxHashSet<BindingId>>,
}

impl LiveValues {
    pub fn of(graph: &BodyGraph) -> Self {
        let mut live = Self {
            live_before: graph.points().map(|_| FxHashSet::default()).collect(),
        };
        while live.grow(graph) {}
        live
    }

    pub fn before(&self, point: PointId) -> &FxHashSet<BindingId> {
        &self.live_before[point.index()]
    }

    fn grow(&mut self, graph: &BodyGraph) -> bool {
        let mut grew = false;
        for (point, facts) in graph.points().rev() {
            let mut live: FxHashSet<BindingId> = facts
                .successors()
                .iter()
                .flat_map(|successor| self.live_before[successor.index()].iter().copied())
                .collect();
            for defined in facts.defined() {
                live.remove(defined);
            }
            live.extend(facts.used().iter().copied());
            if live.len() != self.live_before[point.index()].len() {
                grew = true;
                self.live_before[point.index()] = live;
            }
        }
        grew
    }
}

#[cfg(test)]
mod tests {
    use super::LiveValues;
    use crate::core::BindingId;
    use crate::ownership::loans::body_graph::BodyGraph;
    use crate::ownership::loans::point_id::PointId;

    #[test]
    fn a_value_is_live_from_its_definition_to_its_last_use_and_around_a_loop() {
        let value = BindingId::from_index(7);
        let mut graph = BodyGraph::new();
        let defined = graph.join(&[PointId::ENTRY]);
        graph.point_mut(defined).define(value);
        let head = graph.join(&[defined]);
        let used = graph.join(&[head]);
        graph.point_mut(used).using(value);
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
