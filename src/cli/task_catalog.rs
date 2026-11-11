use std::ops::Range;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::diagnostic_output::DiagnosticOutput;
use super::distinct_output::DistinctOutput;
use super::entry_tree::EntryTree;
use super::evaluation_request::EvaluationRequest;
use super::exit_status::ExitStatus;
use super::project_task::TaskName;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use super::user_label_output::UserLabelOutput;
use crate::core::{Expr, ExprKind, Leaf, Literal, ModulePath};
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, Program};
use crate::modules::ModuleSources;
use crate::reader::StringLiteral;
use crate::source::{SourceId, Span};
use crate::types::{TypedExpr, TypedExprKind, TypedLeaf, TypedProgram};

pub struct TaskCatalog {
    request: EvaluationRequest,
    names_lookup: String,
}

pub struct EnteredProgram {
    pub program: Program,
    pub source: SourceId,
}

impl TaskCatalog {
    pub const LOOKUP_SOURCE: &'static str = "<task lookup>";
    pub const RUNNER_SOURCE: &'static str = "<task runner>";
    const LISTING: &'static str = "(task-listing)";
    const BLOCK_START: &'static str = "(do ";
    const BLOCK_END: &'static str = " (do))";
    const SEPARATOR: &'static str = " ";

    pub fn user_output(inner: Rc<dyn DiagnosticOutput>) -> Rc<dyn DiagnosticOutput> {
        let distinct = Rc::new(DistinctOutput::new(inner));
        let generated = vec![
            EntryTree::TASKS_ENTRY,
            Self::LOOKUP_SOURCE,
            Self::RUNNER_SOURCE,
        ];
        Rc::new(UserLabelOutput::new(distinct, generated))
    }

    pub fn load(tree: &EntryTree, terminal: &mut Terminal) -> Result<Self, ExitStatus> {
        let modules: Vec<String> = tree
            .module_paths()?
            .iter()
            .map(|module| StringLiteral::quoted(module))
            .collect();
        Ok(Self {
            request: EvaluationRequest {
                module: tree.load(terminal)?,
                evaluated: Vec::new(),
            },
            names_lookup: format!("(task-names {})", modules.join(Self::SEPARATOR)),
        })
    }

    pub fn names(&self, terminal: &mut Terminal) -> Result<Vec<TaskName>, ExitStatus> {
        let names = self.expanded_text(terminal, &self.names_lookup)?;
        Ok(names
            .lines()
            .map(|name| TaskName::new(name.to_string()))
            .collect())
    }

    pub fn listing(&self, terminal: &mut Terminal) -> Result<String, ExitStatus> {
        self.names(terminal)?;
        let listing = self.expanded_text(terminal, Self::LISTING)?;
        Ok(if listing.is_empty() {
            listing
        } else {
            format!("{listing}\n")
        })
    }

    pub fn signatures(
        &self,
        terminal: &mut Terminal,
        names: &[TaskName],
    ) -> Result<Vec<String>, ExitStatus> {
        let lookups: Vec<String> = names
            .iter()
            .map(|name| format!("(task-signature {})", StringLiteral::quoted(name.as_str())))
            .collect();
        let (block, ranges) = Self::block_of(&lookups);
        let entered = self.entered_program(terminal, &block)?;
        let typed = terminal.checking().check(&entered.program)?;
        let texts = Self::texts_by_span(&typed);
        names
            .iter()
            .zip(ranges)
            .map(|(name, range)| {
                texts
                    .get(&Span::new(entered.source, range))
                    .cloned()
                    .ok_or_else(|| Self::report_no_signature(name))
            })
            .collect()
    }

    pub fn block_of(forms: &[String]) -> (String, Vec<Range<usize>>) {
        let mut block = String::from(Self::BLOCK_START);
        let mut ranges = Vec::new();
        for form in forms {
            block.push_str(Self::SEPARATOR);
            ranges.push(block.len()..block.len() + form.len());
            block.push_str(form);
        }
        block.push_str(Self::BLOCK_END);
        (block, ranges)
    }

    pub fn entered_program(
        &self,
        terminal: &mut Terminal,
        entry: &str,
    ) -> Result<EnteredProgram, ExitStatus> {
        terminal
            .expansion()
            .within_evaluation(&self.request, |mut session| {
                let source = session.add_source(Self::RUNNER_SOURCE, entry);
                Self::entered(session, source).map(|program| EnteredProgram { program, source })
            })
    }

    fn entered(
        mut session: ExpansionSession<'_>,
        source: SourceId,
    ) -> Result<Program, Diagnostics> {
        let entry = session.task_runner_entry(source)?;
        let mut program = session.finish()?;
        program.enter(&entry);
        Ok(program)
    }

    fn texts_by_span(typed: &TypedProgram) -> FxHashMap<Span, String> {
        let entry = typed
            .entry_module()
            .definitions
            .iter()
            .find(|definition| Some(definition.binding) == typed.entry_function);
        let mut texts = FxHashMap::default();
        if let Some(entry) = entry {
            Self::collect_texts(&entry.value, &mut texts);
        }
        texts
    }

    fn collect_texts(expr: &TypedExpr, texts: &mut FxHashMap<Span, String>) {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Literal(Literal::String(text))) => {
                texts.insert(expr.span, text.to_string());
            }
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| Self::collect_texts(child, texts)),
        }
    }

    fn expanded_text(&self, terminal: &mut Terminal, lookup: &str) -> Result<String, ExitStatus> {
        let expanded = self.expanded(terminal, lookup)?;
        Self::literal(&expanded).ok_or_else(|| Self::report_no_string(lookup))
    }

    fn expanded(&self, terminal: &mut Terminal, lookup: &str) -> Result<Expr, ExitStatus> {
        terminal
            .expansion()
            .within_evaluation(&self.request, |mut session| {
                let source = session.add_source(Self::LOOKUP_SOURCE, lookup);
                Self::task_expression(&mut session, source)
            })
    }

    fn task_expression(
        session: &mut ExpansionSession<'_>,
        source: SourceId,
    ) -> Result<Expr, Diagnostics> {
        let form = session.read(source)?.remove(0);
        session.expression_in(&ModulePath::new(ModuleSources::TASK), form)
    }

    fn literal(expanded: &Expr) -> Option<String> {
        match expanded.kind() {
            ExprKind::Leaf(Leaf::Literal(Literal::String(text))) => Some(text.to_string()),
            _ => None,
        }
    }

    fn report_no_string(lookup: &str) -> ExitStatus {
        StandardError::write_line(format_args!("error: `{lookup}` did not expand to a string"));
        ExitStatus::ProgramErrors
    }

    fn report_no_signature(name: &TaskName) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: internal: the signature of task `{}` is not at its lookup",
            name.as_str()
        ));
        ExitStatus::ProgramErrors
    }
}
