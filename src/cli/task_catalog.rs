use std::ops::Range;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::diagnostic_output::DiagnosticOutput;
use super::distinct_output::DistinctOutput;
use super::entry_tree::EntryTree;
use super::evaluation_request::EvaluationRequest;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::project_task::TaskName;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use super::tree_modules::TreeModules;
use super::user_label_output::UserLabelOutput;
use crate::core::{Expr, ExprKind, Leaf, Literal, ModulePath};
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, Program};
use crate::modules::ModuleSources;
use crate::reader::StringLiteral;
use crate::source::{SourceId, Span};
use crate::types::{TypedExpr, TypedExprKind, TypedLeaf, TypedProgram};

struct TaskLookup {
    expression: Expr,
    warnings: Diagnostics,
}

pub struct TaskCatalog {
    request: EvaluationRequest,
    names_lookup: String,
}

pub struct EnteredProgram {
    pub program: Program,
    pub source: SourceId,
    pub file: ProgramFile,
}

impl TaskCatalog {
    pub const LOOKUP_SOURCE: &'static str = "<task lookup>";
    pub const RUNNER_SOURCE: &'static str = "<task runner>";
    const LISTING: &'static str = "(task-listing)";
    const BLOCK_START: &'static str = "(do ";
    const BLOCK_END: &'static str = " (do))";
    const SEPARATOR: &'static str = " ";
    const HIDDEN_FOLDER: &'static str = ":hidden-folder";
    const PROJECT_FILE: &'static str = ":project-file";

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
        let modules = Self::tree_arguments(&tree.module_paths()?);
        Ok(Self {
            request: EvaluationRequest {
                module: tree.load(terminal)?,
                evaluated: Vec::new(),
            },
            names_lookup: format!("(task-names {})", modules.join(Self::SEPARATOR)),
        })
    }

    fn tree_arguments(tree: &TreeModules) -> Vec<String> {
        let modules = tree
            .modules
            .iter()
            .map(|module| StringLiteral::quoted(module));
        let hidden = tree
            .hidden_folders
            .iter()
            .map(|folder| format!("{} {}", Self::HIDDEN_FOLDER, StringLiteral::quoted(folder)));
        let projects = tree
            .project_files
            .iter()
            .map(|file| format!("{} {}", Self::PROJECT_FILE, StringLiteral::quoted(file)));
        modules.chain(hidden).chain(projects).collect()
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
        let lookup = self.expanded(terminal, Self::LISTING)?;
        terminal.source_report().print(&lookup.warnings).passed()?;
        let listing = lookup.text(Self::LISTING)?;
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
        let file = self.request.module.file.clone();
        terminal
            .expansion()
            .sessions()
            .within_evaluation(&self.request, |mut session| {
                let source = session.add_source(Self::RUNNER_SOURCE, entry);
                Self::entered(session, source).map(|program| EnteredProgram {
                    program,
                    source,
                    file,
                })
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
        self.expanded(terminal, lookup)?.text(lookup)
    }

    fn expanded(&self, terminal: &mut Terminal, lookup: &str) -> Result<TaskLookup, ExitStatus> {
        terminal
            .expansion()
            .sessions()
            .within_evaluation(&self.request, |mut session| {
                let source = session.add_source(Self::LOOKUP_SOURCE, lookup);
                Self::task_lookup(session, source)
            })
    }

    fn task_lookup(
        mut session: ExpansionSession<'_>,
        source: SourceId,
    ) -> Result<TaskLookup, Diagnostics> {
        let form = session.read(source)?.remove(0);
        let expression = session.expression_in(&ModulePath::new(ModuleSources::TASK), form)?;
        let warnings = session.finish()?.warnings;
        Ok(TaskLookup {
            expression,
            warnings,
        })
    }

    fn report_no_signature(name: &TaskName) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: internal: the signature of task `{}` is not at its lookup",
            name.as_str()
        ));
        ExitStatus::ProgramErrors
    }
}

impl TaskLookup {
    fn text(&self, lookup: &str) -> Result<String, ExitStatus> {
        Self::literal(&self.expression).ok_or_else(|| Self::report_no_string(lookup))
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
}
