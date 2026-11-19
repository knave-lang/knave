mod access;
mod assoc_flows;
mod body_flows;
mod call_flows;
mod call_target;
mod callee;
mod callee_scheme;
mod class_role;
mod flow;
#[cfg(test)]
mod flow_fixture;
mod flow_graph;
mod inline_flows;
mod inline_rust;
mod instantiation;
mod listed_section;
mod listed_slot;
mod need;
mod node_id;
mod node_partition;
mod reaching_text;
mod representation;
mod scheme_position;
mod scheme_positions;
mod slot;
mod slot_classes;
mod slot_listing;
mod slot_owner;
mod slot_reads;
mod source;
mod stored_parameters;
mod template_call;
mod text_conversion;
mod text_decision_table;
mod text_facts;
mod text_names;
mod text_position;
mod text_program;
mod text_read;
mod text_refusals;
mod text_report;
mod text_traits;
mod text_value;
mod text_walk;
mod transfer;
mod try_flows;
mod type_path;
mod user_function;
mod walk_context;

use slot_classes::SlotClasses;
use text_decision_table::TextDecisionTable;
use text_facts::TextFacts;
use text_program::TextProgram;
use text_refusals::TextRefusals;
use text_report::TextReport;
use text_walk::TextWalk;

use crate::source::SourceMap;
use crate::types::TypedProgram;

pub struct TextDecisions<'p> {
    program: TextProgram<'p>,
}

impl<'p> TextDecisions<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program: TextProgram::of(program),
        }
    }

    pub fn report(&self, sources: &SourceMap) -> String {
        let (graph, listing) = TextWalk::of(&self.program);
        let classes = SlotClasses::of(&graph);
        let facts = TextFacts::spread(&classes);
        let table = TextDecisionTable::decide(&graph, &classes, &facts);
        let refusals = TextRefusals::new(&classes, &facts, &table).found();
        TextReport::new(&self.program, &table, &refusals).written(&listing, sources)
    }
}
