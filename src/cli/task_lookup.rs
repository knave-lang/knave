use std::path::PathBuf;

use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::standard_error::StandardError;
use super::terminal::{EvaluationRequest, Terminal};
use super::test_entries::EntryTree;
use crate::build::BuildProfile;
use crate::core::{Expr, ExprKind, Leaf, Literal, ModulePath};
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, Program};
use crate::modules::ModuleSources;
use crate::source::SourceId;
use crate::types::{TypedExpr, TypedExprKind, TypedLeaf, TypedProgram};

pub struct TaskLookup {
    request: EvaluationRequest,
    names_lookup: String,
}

impl TaskLookup {
    const LOOKUP_SOURCE: &'static str = "<task lookup>";
    const RUNNER_SOURCE: &'static str = "<task runner>";
    const LISTING: &'static str = "(task-listing)";
    const NO_RUNNER: &'static str = "(do)";

    pub fn load(tree: &EntryTree, terminal: &mut Terminal) -> Result<Self, ExitStatus> {
        let modules: Vec<String> = tree
            .module_paths()?
            .iter()
            .map(|module| format!("{module:?}"))
            .collect();
        Ok(Self {
            request: EvaluationRequest {
                module: tree.load(terminal)?,
                evaluated: Vec::new(),
            },
            names_lookup: format!("(task-names {})", modules.join(" ")),
        })
    }

    pub fn names(&self, terminal: &mut Terminal) -> Result<Vec<String>, ExitStatus> {
        let names = self.expanded_text(terminal, &self.names_lookup)?;
        Ok(names.lines().map(str::to_string).collect())
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

    pub fn build(
        &self,
        terminal: &mut Terminal,
        project: &LocatedProject,
        name: &str,
        profile: BuildProfile,
    ) -> Result<PathBuf, ExitStatus> {
        let signatures = self.signatures(terminal, &[name.to_string()])?;
        let program = self.entered_program(terminal, &Self::runner(name, &signatures[0]))?;
        let checked = terminal.check_expanded(&program)?;
        let emitted = terminal.emit_checked(&checked)?;
        terminal.build_task(&emitted, project, name, profile)
    }

    pub fn check(&self, terminal: &mut Terminal) -> Result<(), ExitStatus> {
        let names = self.names(terminal)?;
        let signatures = self.signatures(terminal, &names)?;
        if names.is_empty() {
            return self.check_entry(terminal, Self::NO_RUNNER);
        }
        names
            .iter()
            .zip(&signatures)
            .map(|(name, signature)| self.check_entry(terminal, &Self::runner(name, signature)))
            .fold(Ok(()), Result::and)
    }

    fn check_entry(&self, terminal: &mut Terminal, entry: &str) -> Result<(), ExitStatus> {
        let program = self.entered_program(terminal, entry)?;
        terminal.check_expanded(&program).map(|_| ())
    }

    fn runner(name: &str, signature: &str) -> String {
        format!("(run-task {name:?} {signature:?})")
    }

    fn signatures(
        &self,
        terminal: &mut Terminal,
        names: &[String],
    ) -> Result<Vec<String>, ExitStatus> {
        let mut signatures = Vec::new();
        let mut open = Vec::new();
        for (index, name) in names.iter().enumerate() {
            let lookup = Self::signature_lookup(name);
            match Self::literal(&self.expanded(terminal, &lookup)?) {
                Some(known) => signatures.push(known),
                None => {
                    signatures.push(String::new());
                    open.push((index, lookup));
                }
            }
        }
        for ((index, _), inferred) in open.iter().zip(self.inferred_signatures(terminal, &open)?) {
            signatures[*index] = inferred;
        }
        Ok(signatures)
    }

    fn signature_lookup(name: &str) -> String {
        format!("(task-signature {name:?})")
    }

    fn inferred_signatures(
        &self,
        terminal: &mut Terminal,
        open: &[(usize, String)],
    ) -> Result<Vec<String>, ExitStatus> {
        if open.is_empty() {
            return Ok(Vec::new());
        }
        let lookups: Vec<&str> = open.iter().map(|(_, lookup)| lookup.as_str()).collect();
        let program =
            self.entered_program(terminal, &format!("(do {} (do))", lookups.join(" ")))?;
        let typed = terminal.check_expanded(&program)?;
        let texts = Self::inferred_texts(&typed);
        if texts.len() == open.len() {
            Ok(texts)
        } else {
            Err(Self::report_no_string(lookups[0]))
        }
    }

    fn entered_program(&self, terminal: &mut Terminal, entry: &str) -> Result<Program, ExitStatus> {
        terminal.hide_labels_in_generated(Self::RUNNER_SOURCE);
        terminal.within_evaluation(&self.request, |mut session| {
            let source = session.add_source(Self::RUNNER_SOURCE, entry);
            Self::entered(session, source)
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

    fn task_expression(
        session: &mut ExpansionSession<'_>,
        source: SourceId,
    ) -> Result<Expr, Diagnostics> {
        let form = session.read(source)?.remove(0);
        session.expression_in(&ModulePath::new(ModuleSources::TASK), form)
    }

    fn inferred_texts(typed: &TypedProgram) -> Vec<String> {
        let entry = typed
            .entry_module()
            .definitions
            .iter()
            .find(|definition| Some(definition.binding) == typed.entry_function);
        let mut texts = Vec::new();
        if let Some(entry) = entry {
            Self::collect_texts(&entry.value, &mut texts);
        }
        texts
    }

    fn collect_texts(expr: &TypedExpr, texts: &mut Vec<String>) {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Literal(Literal::String(text))) => {
                texts.push(text.to_string())
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
        terminal.hide_labels_in_generated(Self::LOOKUP_SOURCE);
        terminal.within_evaluation(&self.request, |mut session| {
            let source = session.add_source(Self::LOOKUP_SOURCE, lookup);
            Self::task_expression(&mut session, source)
        })
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
