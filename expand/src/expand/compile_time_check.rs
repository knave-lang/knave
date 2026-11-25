use super::Expander;
use super::globals::CompileTimeCheck;
use super::macro_call_lookup::MacroCallLookup;
use super::problems::ExpandProblems;
use crate::core::{
    BindingId, BindingKind, CollectionForm, Control, Expr, ExprKind, ExpressionForm, KernelForm,
    Lambda, Leaf,
};
use crate::diagnostic::{Diagnostic, Label};
use crate::interpret::InterpretProblems;
use crate::source::Span;

mod kernel_form_problems;

use kernel_form_problems::KernelFormProblems;

impl Expander<'_> {
    pub(super) fn check_compile_time(&mut self, lambda: &Lambda) -> Result<(), Diagnostic> {
        for expr in lambda.body.expressions() {
            self.check_expr(expr, None)?;
        }
        Ok(())
    }

    fn check_expr(&mut self, expr: &Expr, through: Option<Span>) -> Result<(), Diagnostic> {
        self.check_node(expr, through)?;
        for child in expr.children() {
            self.check_expr(child, through)?;
        }
        Ok(())
    }

    fn check_node(&mut self, expr: &Expr, through: Option<Span>) -> Result<(), Diagnostic> {
        match expr.kind() {
            ExprKind::Leaf(Leaf::Var(binding)) => self.check_global(*binding, expr.span(), through),
            ExprKind::Leaf(Leaf::Rust(_)) => {
                let problem = self
                    .macro_program_only(expr.span())
                    .unwrap_or_else(|| InterpretProblems::program_only(expr.span()));
                Err(Self::routed(problem, through))
            }
            ExprKind::SetAt(path) if path.names_the_whole_place() => {
                let set_at =
                    KernelForm::Expression(ExpressionForm::Collection(CollectionForm::SetAt));
                let problem = self.macro_program_only(expr.span()).unwrap_or_else(|| {
                    KernelFormProblems::program_only(set_at.name(), expr.span())
                });
                Err(Self::routed(problem, through))
            }
            _ => self.check_value_form(expr, through),
        }
    }

    fn macro_program_only(&self, span: Span) -> Option<Diagnostic> {
        let origin = self.tables.macro_origins.calls.introducing_macro(span)?;
        Some(ExpandProblems::macro_program_only(
            &origin.name,
            span,
            origin.defined,
        ))
    }

    fn check_value_form(&self, expr: &Expr, through: Option<Span>) -> Result<(), Diagnostic> {
        let problem = match expr.kind() {
            ExprKind::Control(Control::Match(..)) => {
                InterpretProblems::match_only_at_runtime(expr.span())
            }
            kind => match Self::runtime_only_feature(kind) {
                Some(feature) => InterpretProblems::runtime_only(feature, expr.span()),
                None => return Ok(()),
            },
        };
        Err(Self::routed(problem, through))
    }

    fn runtime_only_feature(kind: &ExprKind) -> Option<&'static str> {
        match kind {
            ExprKind::Struct(_) => Some("structs"),
            ExprKind::Assoc(..) | ExprKind::SetAt(..) => Some("collection updates"),
            ExprKind::MapAt(..) => Some("map operations"),
            ExprKind::PushAt(..) => Some("collection pushes"),
            ExprKind::Try(_) => Some("`try` forms"),
            ExprKind::Infer(_) => Some("`infer` forms"),
            _ => None,
        }
    }

    fn check_global(
        &mut self,
        binding: BindingId,
        span: Span,
        through: Option<Span>,
    ) -> Result<(), Diagnostic> {
        if !matches!(self.kind(binding), BindingKind::Global(_)) {
            return Ok(());
        }
        let value = self.lowered_global(binding, span)?;
        let name = self.tables.bindings.get(binding).name().to_string();
        if let ExprKind::Leaf(Leaf::Rust(_)) = value.kind() {
            let problem = self
                .macro_program_only(span)
                .unwrap_or_else(|| ExpandProblems::program_only(&name, span, value.span()));
            return Err(Self::routed(problem, through));
        }
        match self.tables.globals.compile_time_check(binding) {
            Some(CompileTimeCheck::InProgress | CompileTimeCheck::Passed) => Ok(()),
            Some(CompileTimeCheck::Failed) => Err(ExpandProblems::broken_definition(&name, span)),
            None => self.check_helper(binding, &value, through.or(Some(span))),
        }
    }

    fn check_helper(
        &mut self,
        binding: BindingId,
        value: &Expr,
        through: Option<Span>,
    ) -> Result<(), Diagnostic> {
        self.tables
            .globals
            .record_compile_time_check(binding, CompileTimeCheck::InProgress);
        let checked = self.check_expr(value, through);
        let outcome = if checked.is_ok() {
            CompileTimeCheck::Passed
        } else {
            CompileTimeCheck::Failed
        };
        self.tables
            .globals
            .record_compile_time_check(binding, outcome);
        checked
    }

    fn routed(problem: Diagnostic, through: Option<Span>) -> Diagnostic {
        match through {
            Some(span) => {
                problem.with_label(Label::new(span, "macro code reaches it through this"))
            }
            None => problem,
        }
    }
}
