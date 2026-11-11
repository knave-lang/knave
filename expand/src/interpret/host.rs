use super::definition_queries::DefinitionQueries;
use super::expansion_callbacks::ExpansionCallbacks;
use super::global_values::GlobalValues;
use super::module_notes::ModuleNotes;
use super::module_queries::ModuleQueries;
use super::name_comparison::NameComparison;
use super::source_queries::SourceQueries;
use super::warnings::Warnings;

pub trait Host {
    fn global_values(&mut self) -> Option<&mut dyn GlobalValues>;
    fn module_queries(&mut self) -> Option<&mut dyn ModuleQueries>;
    fn definition_queries(&mut self) -> Option<&mut dyn DefinitionQueries>;
    fn module_notes(&mut self) -> Option<&mut dyn ModuleNotes>;
    fn name_comparison(&mut self) -> Option<&mut dyn NameComparison>;
    fn expansion_callbacks(&mut self) -> Option<&mut dyn ExpansionCallbacks>;
    fn source_queries(&mut self) -> Option<&mut dyn SourceQueries>;
    fn warnings(&mut self) -> Option<&mut dyn Warnings>;
}
