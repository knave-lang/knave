use super::body_facts::BodyFacts;
use super::closure::Closure;
use super::control_flow_graph::ControlFlowGraph;
use super::holdings::Holdings;
use super::lender::Lender;
use super::loan_table::LoanTable;
use super::loop_heads::LoopHeads;
use super::loop_scope::LoopScope;
use super::point_id::PointId;
use super::value_regions::ValueRegions;
use crate::core::BindingId;
use crate::types::TypedBinder;

#[derive(Default)]
pub struct BodyBuilder<'w> {
    graph: ControlFlowGraph,
    loans: LoanTable,
    regions: ValueRegions,
    loops: LoopHeads<'w>,
}

impl<'w> BodyBuilder<'w> {
    pub fn join(&mut self, before: &[PointId]) -> PointId {
        self.graph.join(before)
    }

    pub fn read(&mut self, at: PointId, binding: BindingId) -> PointId {
        let point = self.graph.join(&[at]);
        self.graph.point_mut(point).using(binding);
        point
    }

    pub fn define(&mut self, point: PointId, binder: &TypedBinder, held: &Holdings) {
        self.graph.point_mut(point).define(binder.binding);
        self.regions.flow_into(binder.binding, held);
    }

    pub fn lender(&mut self, point: PointId) -> Lender<'_> {
        Lender::new(&mut self.graph, &mut self.loans, point)
    }

    pub fn loops(&mut self) -> LoopScope<'_, 'w> {
        LoopScope::new(&mut self.graph, &mut self.loops)
    }

    pub fn into_facts(self, returned: Holdings, closures: Vec<Closure<'w>>) -> BodyFacts<'w> {
        BodyFacts::new(self.graph, self.loans, self.regions)
            .returning(returned)
            .with_closures(closures)
    }
}

#[cfg(test)]
mod tests {
    use super::BodyBuilder;
    use crate::ownership::loans::point_id::PointId;

    #[test]
    fn a_loop_is_the_innermost_from_its_entry_until_it_is_left() {
        let mut builder = BodyBuilder::default();
        let head = builder.join(&[PointId::ENTRY]);
        assert!(builder.loops().innermost_parameters().is_none());
        builder.loops().enter(head, &[]);
        assert_eq!(
            builder.loops().innermost_parameters().map(<[_]>::len),
            Some(0)
        );
        builder.loops().leave();
        assert!(builder.loops().innermost_parameters().is_none());
    }
}
