mod access_facts;
mod argument_passing;
mod body_graph;
mod body_walk;
mod call_site;
mod holdings;
mod lending;
mod live_values;
mod loan;
mod loan_context;
mod loan_facts;
mod loan_id;
mod loan_names;
mod loan_place;
#[cfg(test)]
mod loan_source;
mod place_access;
mod point_id;
mod program_point;
mod result_loans;
mod value_regions;
mod walked_body;

pub use loan_facts::LoanFacts;
