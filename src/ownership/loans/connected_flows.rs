use rustc_hash::{FxHashMap, FxHashSet};

use super::recorded_flow::RecordedFlow;
use crate::core::BindingId;

#[derive(Default)]
pub struct ConnectedFlows {
    parents: FxHashMap<BindingId, BindingId>,
}

impl ConnectedFlows {
    pub fn kept(flows: Vec<RecordedFlow>, targets: &[BindingId]) -> Vec<RecordedFlow> {
        let mut connected = Self::default();
        for flow in &flows {
            for source in flow.sources() {
                connected.unite(flow.binding, source);
            }
        }
        let roots: FxHashSet<BindingId> = targets
            .iter()
            .map(|target| connected.root(*target))
            .collect();
        flows
            .into_iter()
            .filter(|flow| roots.contains(&connected.root(flow.binding)))
            .collect()
    }

    fn unite(&mut self, first: BindingId, second: BindingId) {
        let (first, second) = (self.root(first), self.root(second));
        if first != second {
            self.parents.insert(first, second);
        }
    }

    fn root(&mut self, binding: BindingId) -> BindingId {
        let Some(parent) = self.parents.get(&binding).copied() else {
            return binding;
        };
        let root = self.root(parent);
        self.parents.insert(binding, root);
        root
    }
}

#[cfg(test)]
mod tests {
    use super::ConnectedFlows;
    use crate::core::BindingId;
    use crate::ownership::loans::point_id::PointId;
    use crate::ownership::loans::recorded_flow::RecordedFlow;

    struct ConnectedFixture;

    impl ConnectedFixture {
        fn flow(binding: u32, sources: &[u32]) -> RecordedFlow {
            let sources: Vec<BindingId> =
                sources.iter().copied().map(BindingId::from_index).collect();
            RecordedFlow::from_sources(PointId::ENTRY, BindingId::from_index(binding), &sources)
        }
    }

    #[test]
    fn only_flows_linked_to_a_target_through_their_sources_are_kept() {
        let flows = vec![
            ConnectedFixture::flow(1, &[]),
            ConnectedFixture::flow(2, &[1]),
            ConnectedFixture::flow(3, &[2]),
            ConnectedFixture::flow(4, &[]),
            ConnectedFixture::flow(5, &[4]),
        ];
        let kept = ConnectedFlows::kept(flows, &[BindingId::from_index(3)]);
        let bindings: Vec<BindingId> = kept.iter().map(|flow| flow.binding).collect();
        assert_eq!(bindings, [1, 2, 3].map(BindingId::from_index).to_vec());
    }
}
