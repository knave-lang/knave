mod access;
mod body_flows;
mod call_flows;
mod callee;
mod flow;
mod flow_graph;
mod inline_flows;
mod listed_section;
mod listed_slot;
mod need;
mod node_id;
mod reaching_text;
mod representation;
mod scheme_position;
mod scheme_positions;
mod slot;
mod slot_listing;
mod slot_owner;
mod slot_reads;
mod source;
mod stored_parameters;
mod text_conversion;
mod text_names;
mod text_position;
mod text_program;
mod text_report;
mod text_solver;
mod text_value;
mod text_walk;
mod transfer;
mod type_path;
mod walk_context;

use text_program::TextProgram;
use text_report::TextReport;
use text_solver::TextSolver;
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
        let solver = TextSolver::solve(&graph);
        TextReport::new(&self.program, &graph, &solver).written(&listing, sources)
    }
}
