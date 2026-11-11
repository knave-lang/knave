use super::definition_queries::DefinitionQueries;
use super::expansion_callbacks::ExpansionCallbacks;
use super::module_notes::ModuleNotes;
use super::module_queries::ModuleQueries;
use super::name_comparison::NameComparison;
use super::source_queries::SourceQueries;
use super::warnings::Warnings;

pub trait CompileTimeHost:
    ModuleQueries
    + DefinitionQueries
    + ModuleNotes
    + NameComparison
    + ExpansionCallbacks
    + SourceQueries
    + Warnings
{
}

impl<Host> CompileTimeHost for Host where
    Host: ModuleQueries
        + DefinitionQueries
        + ModuleNotes
        + NameComparison
        + ExpansionCallbacks
        + SourceQueries
        + Warnings
{
}
