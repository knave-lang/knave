mod access;
mod assoc_flows;
mod binding_owners;
mod body_flows;
mod call_flows;
mod call_result_reads;
mod call_results;
mod call_target;
mod callee;
mod callee_scheme;
mod class_role;
mod closure_slots;
mod compound_queries;
mod construction_flows;
mod enum_payloads;
mod flow;
#[cfg(test)]
mod flow_fixture;
mod flow_graph;
mod function_value;
mod global_reads;
mod inline_flows;
mod inline_rust;
mod instantiation;
mod listed_section;
mod listed_slot;
mod map_queries;
mod need;
mod node_id;
mod node_partition;
mod pattern_flows;
mod place_writes;
mod reaching_text;
mod refusal;
mod representation;
mod scheme_position;
mod scheme_positions;
mod short_lived_text;
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
mod text_types;
mod text_value;
mod text_walk;
mod trait_method_slots;
mod transfer;
mod try_flows;
mod type_path;
mod user_function;
mod walk_context;
mod written_bindings;

use call_results::CallResults;
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
        let calls = CallResults::of(&graph, &classes);
        let facts = TextFacts::spread(&classes, &calls);
        let table = TextDecisionTable::decide(&graph, &classes, &facts);
        let refusals = TextRefusals::new(&classes, &facts, &table).found();
        TextReport::new(&self.program, &table, &refusals).written(&listing, sources)
    }
}
