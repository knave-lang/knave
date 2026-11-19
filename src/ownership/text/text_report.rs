use rustc_hash::FxHashMap;

use super::flow_graph::FlowGraph;
use super::node::NodeId;
use super::representation::Representation;
use super::slot::{ListedSlot, Slot, SlotOwner};
use super::slot_listing::{ListedSection, SlotListing};
use super::text_program::TextProgram;
use super::text_solver::TextSolver;
use crate::source::{SourceMap, Span};

pub struct TextReport<'r, 'p> {
    program: &'r TextProgram<'p>,
    solver: &'r TextSolver<'r>,
    nodes: FxHashMap<&'r Slot, NodeId>,
}

impl<'r, 'p> TextReport<'r, 'p> {
    pub fn new(
        program: &'r TextProgram<'p>,
        graph: &'r FlowGraph,
        solver: &'r TextSolver<'r>,
    ) -> Self {
        let nodes = graph
            .classes()
            .into_values()
            .flatten()
            .map(|node| (graph.slot(node), node))
            .collect();
        Self {
            program,
            solver,
            nodes,
        }
    }

    pub fn written(&self, listing: &SlotListing, sources: &SourceMap) -> String {
        let sections: String = listing
            .sections()
            .iter()
            .map(|section| self.section(section))
            .collect();
        sections + &self.refusals(sources)
    }

    fn section(&self, section: &ListedSection) -> String {
        let lines: String = section
            .slots
            .iter()
            .map(|listed| format!("  {}\n", self.line(listed)))
            .collect();
        match &section.heading {
            Some(heading) if !lines.is_empty() => format!("{heading}\n{lines}"),
            _ => String::new(),
        }
    }

    fn line(&self, listed: &ListedSlot) -> String {
        let decided = self.decided(&listed.slot);
        let label = format!("{}{}", self.label(&listed.slot.owner), listed.slot.path);
        if decided == listed.written {
            format!("{label}: {decided}")
        } else {
            format!("{label}: {decided}, written {}", listed.written)
        }
    }

    fn decided(&self, slot: &Slot) -> Representation {
        match self.nodes.get(slot) {
            Some(node) => self.solver.representation(self.solver.class(*node)),
            None if slot.is_stored_place() => Representation::Owned,
            None => Representation::View,
        }
    }

    fn label(&self, owner: &SlotOwner) -> String {
        match owner {
            SlotOwner::Parameter(binding) => self.program.names().name(*binding).to_string(),
            SlotOwner::Result(_) => "result".to_string(),
            SlotOwner::Local(binding) => format!("local {}", self.program.names().name(*binding)),
            SlotOwner::Field(_, name) => name.to_string(),
            SlotOwner::Variable(..)
            | SlotOwner::Value(_)
            | SlotOwner::OwnedArgument(_)
            | SlotOwner::ClonedArgument(_)
            | SlotOwner::WrittenView(..) => String::new(),
        }
    }

    fn refusals(&self, sources: &SourceMap) -> String {
        let mut spans: Vec<Span> = self
            .solver
            .refusals()
            .into_iter()
            .map(|flow| flow.span)
            .collect();
        spans.sort_by_key(|span| (span.source().index(), span.range().start));
        spans.dedup();
        let lines: String = spans
            .into_iter()
            .map(|span| format!("  {}\n", sources.start_position(span)))
            .collect();
        if lines.is_empty() {
            String::new()
        } else {
            format!("needs owned text\n{lines}")
        }
    }
}
