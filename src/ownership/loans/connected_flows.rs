use rustc_hash::{FxHashMap, FxHashSet};

use super::recorded_flow::RecordedFlow;
use super::recorded_store::RecordedStore;
use super::stored_loans::StoredLoans;
use crate::core::BindingId;

#[derive(Default)]
pub struct ConnectedFlows {
    parents: FxHashMap<BindingId, BindingId>,
}

impl ConnectedFlows {
    pub fn kept(
        flows: Vec<RecordedFlow>,
        stores: &[RecordedStore<StoredLoans>],
    ) -> Vec<RecordedFlow> {
        let mut connected = Self::default();
        for flow in &flows {
            for source in flow.sources() {
                connected.unite(flow.binding, source);
            }
        }
        for store in stores {
            for source in &store.held.sources {
                connected.unite(store.target, *source);
            }
        }
        let roots: FxHashSet<BindingId> = stores
            .iter()
            .map(|store| connected.root(store.target))
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
    use std::collections::BTreeSet;

    use super::ConnectedFlows;
    use crate::core::BindingId;
    use crate::ownership::loans::point_id::PointId;
    use crate::ownership::loans::recorded_flow::RecordedFlow;
    use crate::ownership::loans::recorded_store::RecordedStore;
    use crate::ownership::loans::stored_loans::StoredLoans;

    struct ConnectedFixture;

    impl ConnectedFixture {
        fn binding(index: u32) -> BindingId {
            BindingId::from_index(index)
        }

        fn flow(binding: u32, sources: &[u32]) -> RecordedFlow {
            let sources: Vec<BindingId> = sources.iter().copied().map(Self::binding).collect();
            RecordedFlow::from_sources(PointId::ENTRY, Self::binding(binding), &sources)
        }

        fn store(target: u32, sources: &[u32]) -> RecordedStore<StoredLoans> {
            RecordedStore {
                point: PointId::ENTRY,
                target: Self::binding(target),
                held: StoredLoans {
                    sources: sources.iter().copied().map(Self::binding).collect(),
                    loans: BTreeSet::new(),
                },
            }
        }

        fn kept(flows: Vec<RecordedFlow>, stores: &[RecordedStore<StoredLoans>]) -> Vec<BindingId> {
            ConnectedFlows::kept(flows, stores)
                .iter()
                .map(|flow| flow.binding)
                .collect()
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
        let kept = ConnectedFixture::kept(flows, &[ConnectedFixture::store(3, &[])]);
        assert_eq!(kept, [1, 2, 3].map(BindingId::from_index).to_vec());
    }

    #[test]
    fn a_value_stored_into_a_target_links_its_sources_to_the_target() {
        let flows = vec![
            ConnectedFixture::flow(1, &[]),
            ConnectedFixture::flow(2, &[]),
        ];
        let kept = ConnectedFixture::kept(flows, &[ConnectedFixture::store(2, &[1])]);
        assert_eq!(kept, [1, 2].map(BindingId::from_index).to_vec());
    }
}
