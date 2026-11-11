use super::noted_term::NotedTerm;
use crate::core::ModuleId;

pub(super) struct NotedTerms {
    pub module: ModuleId,
    pub terms: Vec<NotedTerm>,
}
