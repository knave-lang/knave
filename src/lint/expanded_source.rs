use std::path::PathBuf;

use super::bound_names::BoundNames;
use super::linter::Linter;
use super::read_back::ReadBack;
use super::read_back_definition::ReadBackDefinition;
use super::rule_program::RuleProgram;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostic;
use crate::expand::{Expander, Program};
use crate::interpret::StepBudget;
use crate::modules::ModuleSources;
use crate::reader::Form;
use crate::scope::ScopeSupply;
use crate::source::{SourceId, SourceMap, Span};

pub struct ExpandedSource {
    sources: SourceMap,
    program: Program,
    entry: SourceId,
}

impl ExpandedSource {
    const MODULE: &str = "fixture";

    pub fn expand(text: &str) -> Self {
        let mut sources = SourceMap::default();
        let entry = sources.add(Self::MODULE, text);
        let module_sources = ModuleSources::new(PathBuf::from("."), PathBuf::from("."));
        let program = Expander::new(&mut sources, module_sources)
            .expand(ModulePath::new(Self::MODULE), entry)
            .expect("the source expands");
        Self {
            sources,
            program,
            entry,
        }
    }

    pub fn program(&self) -> &Program {
        &self.program
    }

    pub fn sources(&self) -> &SourceMap {
        &self.sources
    }

    pub fn span_of(&self, text: &str) -> Span {
        let start = self
            .sources
            .file(self.entry)
            .text()
            .rfind(text)
            .expect("the source holds the text");
        Span::new(self.entry, start..start + text.len())
    }

    pub fn supply(&mut self) -> &mut ScopeSupply {
        &mut self.program.scopes
    }

    pub fn read_back(&mut self, name: &str) -> String {
        self.read_back_named(name).0.form.to_string()
    }

    pub fn read_back_named(&mut self, name: &str) -> (ReadBackDefinition, BoundNames) {
        let mut read_back = ReadBack::new(&self.program.bindings, &mut self.program.scopes);
        let definition = read_back
            .definitions(self.program.modules.iter())
            .into_iter()
            .find(|definition| definition.name.as_ref() == name)
            .expect("the source defines the name");
        (definition, read_back.into_names())
    }

    pub fn table_rows(&mut self, name: &str) -> Vec<String> {
        let (definition, _) = self.read_back_named(name);
        let span = definition.form.span();
        let table = definition
            .table
            .into_value(span)
            .expect("the table nests within the limit");
        let rows = table
            .as_form()
            .and_then(Form::as_list)
            .expect("the table is a list");
        rows.iter().map(Self::row_text).collect()
    }

    fn row_text(row: &Form) -> String {
        let parts = row.as_list().expect("each row is a list");
        format!(
            "{} {} {} {} {}",
            parts[0], parts[1], parts[2], parts[3], parts[4]
        )
    }

    pub fn warning_titles(&self) -> Vec<String> {
        self.warning_titles_within(StepBudget::DEFAULT_LIMIT)
    }

    pub fn warning_titles_within(&self, step_limit: u64) -> Vec<String> {
        self.warnings_within(step_limit)
            .iter()
            .map(|warning| warning.title().to_string())
            .collect()
    }

    pub fn warning_codes_within(&self, step_limit: u64) -> Vec<String> {
        self.warnings_within(step_limit)
            .iter()
            .filter_map(|warning| Some(warning.code()?.as_str().to_string()))
            .collect()
    }

    fn warnings_within(&self, step_limit: u64) -> Vec<Diagnostic> {
        let mut rules = RuleProgram::expand_from_sources().expect("the lint rules expand");
        Linter::new(&self.program, &self.sources)
            .with_step_limit(step_limit)
            .lint(&mut rules)
            .expect("the lint rules run")
            .in_source_order()
            .into_iter()
            .cloned()
            .collect()
    }
}
