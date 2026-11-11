use super::module_of_sources::ModuleOfSources;
use crate::source::Span;

pub struct EvaluationRequest {
    pub module: ModuleOfSources,
    pub evaluated: Vec<Span>,
}
