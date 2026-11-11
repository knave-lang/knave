mod arguments;
mod build_limit;
mod built_value;
mod builtin_host;
mod builtin_site;
mod builtins;
mod code_list;
mod definition_queries;
mod expansion_callbacks;
mod global_values;
mod held_bytes;
mod host;
mod host_problems;
mod limit_problems;
mod limited_text;
mod list_item;
mod list_size;
mod local_bindings;
mod module_notes;
mod module_queries;
mod name_comparison;
mod notes_too_large;
#[cfg(test)]
mod partial_host;
mod planned_list;
mod problems;
mod source_queries;
mod step_budget;
mod value;
mod value_limits;
mod warning_problems;
mod warnings;
mod work_meter;

pub(crate) use build_limit::BuildLimit;
pub use builtins::BuiltinCall;
pub use code_list::{
    CodeList, ListConversion, ListNesting, ListTooLarge, NestedTooDeep, PassedLimit,
};
pub use definition_queries::DefinitionQueries;
pub use expansion_callbacks::ExpansionCallbacks;
pub use global_values::GlobalValues;
pub use held_bytes::HeldBytes;
pub use host::Host;
pub use host_problems::HostProblems;
pub use limit_problems::LimitProblems;
pub(crate) use limited_text::LimitedText;
pub use local_bindings::LocalBindings;
pub use module_notes::ModuleNotes;
pub use module_queries::ModuleQueries;
pub use name_comparison::NameComparison;
pub use notes_too_large::NotesTooLarge;
pub(crate) use planned_list::PlannedList;
pub use problems::InterpretProblems;
pub use source_queries::SourceQueries;
pub use step_budget::StepBudget;
pub use value::{Closure, Env, Value};
pub use warnings::Warnings;
pub use work_meter::WorkMeter;

use std::rc::Rc;

use crate::core::{BindingId, Block, BodyItem, Control, Expr, ExprKind, Lambda, Leaf, Literal};
use crate::diagnostic::Diagnostic;
use crate::reader::{Form, FormKind};
use crate::source::Span;
use crate::stack::DeepRecursion;

enum Step {
    Value(Value),
    Recur(Vec<Value>),
}

pub struct Interpreter<'h> {
    host: &'h mut dyn Host,
    depth: usize,
    meter: WorkMeter,
    locals: Rc<LocalBindings>,
}

impl<'h> Interpreter<'h> {
    const MAX_DEPTH: usize = 1000;

    pub fn new(host: &'h mut dyn Host, call_site: Span, steps: StepBudget) -> Self {
        Self {
            host,
            depth: 0,
            meter: WorkMeter::new(steps, call_site),
            locals: Rc::default(),
        }
    }

    pub fn call(
        &mut self,
        callee: &Value,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match callee {
            Value::Closure(closure) => self.call_closure(closure, arguments, span),
            Value::Builtin(builtin) => {
                BuiltinCall::new(*builtin, arguments, span, &self.meter).run(self.host)
            }
            other => Err(InterpretProblems::not_callable(other, span)),
        }
    }

    pub fn evaluate(&mut self, expr: &Expr) -> Result<Value, Diagnostic> {
        self.locals = Rc::new(LocalBindings::of_expr(expr));
        self.eval(expr, &Env::default())
    }

    fn eval(&mut self, expr: &Expr, env: &Env) -> Result<Value, Diagnostic> {
        match self.step(expr, env)? {
            Step::Value(value) => Ok(value),
            Step::Recur(..) => unreachable!("expansion allows `recur` only inside a function"),
        }
    }

    fn step(&mut self, expr: &Expr, env: &Env) -> Result<Step, Diagnostic> {
        DeepRecursion::guard(|| self.step_kind(expr, env))
    }

    fn step_kind(&mut self, expr: &Expr, env: &Env) -> Result<Step, Diagnostic> {
        let span = expr.span();
        match expr.kind() {
            ExprKind::Leaf(leaf) => self.eval_leaf(leaf, env, span).map(Step::Value),
            ExprKind::Fn(lambda) => self.closure_over(lambda, env).map(Step::Value),
            ExprKind::Call(callee, arguments) => self
                .eval_call(callee, arguments, env, span)
                .map(Step::Value),
            ExprKind::Recur(arguments) => self.eval_arguments(arguments, env).map(Step::Recur),
            ExprKind::Borrow(_, value) | ExprKind::Deref(value) => {
                self.eval(value, env).map(Step::Value)
            }
            _ => self.value(expr, env, span),
        }
    }

    fn closure_over(&self, lambda: &Rc<Lambda>, env: &Env) -> Result<Value, Diagnostic> {
        self.meter.charge(1 + env.frame_count())?;
        Ok(Value::Closure(Rc::new(Closure {
            lambda: Rc::clone(lambda),
            env: env.clone(),
            locals: Rc::clone(&self.locals),
        })))
    }

    fn value(&mut self, expr: &Expr, env: &Env, span: Span) -> Result<Step, Diagnostic> {
        match expr.kind() {
            ExprKind::Control(control) => self.step_control(control, env, span),
            ExprKind::Struct(_) => Err(InterpretProblems::runtime_only("structs", span)),
            ExprKind::Try(_) => Err(InterpretProblems::runtime_only("`try` forms", span)),
            ExprKind::Variant(..) | ExprKind::LabeledVariant(..) => {
                Err(InterpretProblems::runtime_only("enum values", span))
            }
            _ => unreachable!("the caller matches a value form"),
        }
    }

    fn eval_leaf(&mut self, leaf: &Leaf, env: &Env, span: Span) -> Result<Value, Diagnostic> {
        match leaf {
            Leaf::Literal(literal) => Ok(Self::literal(literal, span)),
            Leaf::Var(binding) => self.variable(*binding, env, span),
            Leaf::Quote(form) => Ok(Value::Form(form.clone())),
            Leaf::Rust(_) => Err(InterpretProblems::program_only(span)),
        }
    }

    fn step_control(
        &mut self,
        control: &Control,
        env: &Env,
        span: Span,
    ) -> Result<Step, Diagnostic> {
        match control {
            Control::Block(block) => self.step_block(block, env),
            Control::If(test, then, otherwise) => self.step_if(test, then, otherwise, env),
            Control::Match(..) => Err(InterpretProblems::match_only_at_runtime(span)),
        }
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        env: &Env,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let function = self.eval(callee, env)?;
        let values = self.eval_arguments(arguments, env)?;
        self.call(&function, values, span)
    }

    fn eval_arguments(&mut self, arguments: &[Expr], env: &Env) -> Result<Vec<Value>, Diagnostic> {
        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            values.push(self.eval(argument, env)?);
        }
        Ok(values)
    }

    fn call_closure(
        &mut self,
        closure: &Closure,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let params = &closure.lambda.params;
        if params.len() != arguments.len() {
            return Err(InterpretProblems::closure_argument_count(
                params.len(),
                arguments.len(),
                span,
            ));
        }
        self.spend_body(closure)?;
        if self.depth >= Self::MAX_DEPTH {
            return Err(InterpretProblems::too_deep(span));
        }
        let env = Self::bind_arguments(closure, arguments);
        let caller_locals = std::mem::replace(&mut self.locals, Rc::clone(&closure.locals));
        self.depth += 1;
        let result = self.run_body(closure, env);
        self.depth -= 1;
        self.locals = caller_locals;
        result
    }

    fn run_body(&mut self, closure: &Closure, mut env: Env) -> Result<Value, Diagnostic> {
        loop {
            match self.step_block(&closure.lambda.body, &env) {
                Ok(Step::Value(value)) => return Ok(value),
                Ok(Step::Recur(arguments)) => {
                    self.spend_body(closure)?;
                    env = Self::bind_arguments(closure, arguments);
                }
                Err(diagnostic) => return Err(diagnostic),
            }
        }
    }

    #[inline]
    fn spend_body(&self, closure: &Closure) -> Result<(), Diagnostic> {
        self.meter.charge(closure.lambda.body_size)
    }

    fn bind_arguments(closure: &Closure, arguments: Vec<Value>) -> Env {
        closure
            .lambda
            .params
            .iter()
            .zip(arguments)
            .fold(closure.env.clone(), |env, (param, value)| {
                env.bind(param.binding, value)
            })
    }

    fn step_block(&mut self, block: &Block, env: &Env) -> Result<Step, Diagnostic> {
        let mut env = env.clone();
        let mut last = Step::Value(Value::Unit);
        for item in &block.items {
            last = match item {
                BodyItem::Let(binder, value) => {
                    let value = self.eval(value, &env)?;
                    env = env.bind(binder.binding, value);
                    Step::Value(Value::Unit)
                }
                BodyItem::Expr(expr) => self.step(expr, &env)?,
            };
        }
        Ok(last)
    }

    fn step_if(
        &mut self,
        test: &Expr,
        then: &Expr,
        otherwise: &Expr,
        env: &Env,
    ) -> Result<Step, Diagnostic> {
        match self.eval(test, env)? {
            Value::Bool(true) => self.step(then, env),
            Value::Bool(false) => self.step(otherwise, env),
            other => Err(InterpretProblems::not_a_boolean(&other, test.span())),
        }
    }

    fn variable(&mut self, binding: BindingId, env: &Env, span: Span) -> Result<Value, Diagnostic> {
        match env.lookup_local(binding, &self.locals) {
            Some(found) => {
                self.meter.charge(found.depth)?;
                Ok(found.value.clone())
            }
            None => self
                .host
                .global_values()
                .ok_or_else(|| HostProblems::missing_global_values(span))?
                .global_value(binding, span),
        }
    }

    fn literal(literal: &Literal, span: Span) -> Value {
        match literal {
            Literal::Number(number) => {
                Value::Form(Form::new(FormKind::Number(number.clone()), span))
            }
            Literal::String(text) => {
                Value::Form(Form::new(FormKind::String(Rc::clone(text)), span))
            }
            Literal::Bool(value) => Value::Bool(*value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::partial_host::PartialHost;
    use super::{Interpreter, StepBudget};
    use crate::core::{BindingArena, BindingInfo, BindingKind, Expr, ExprKind, Leaf, ModuleId};
    use crate::diagnostic::{Diagnostic, DiagnosticCode};
    use crate::source::{SourceId, Span};

    struct GlobalReadFixture;

    impl GlobalReadFixture {
        fn span() -> Span {
            Span::new(SourceId::new(0), 3..9)
        }

        fn call_site() -> Span {
            Span::new(SourceId::new(0), 20..26)
        }

        fn read_without_global_values() -> Diagnostic {
            let binding = BindingArena::default().add(BindingInfo::new(
                "origin".into(),
                BindingKind::Global(ModuleId::new(0)),
                None,
            ));
            let read = Expr::new(ExprKind::Leaf(Leaf::Var(binding)), Self::span());
            Interpreter::new(
                &mut PartialHost::offering_nothing(),
                Self::call_site(),
                StepBudget::full(),
            )
            .evaluate(&read)
            .expect_err("the host offers no global values")
        }
    }

    #[test]
    fn a_global_read_without_global_values_is_refused_at_the_read() {
        let refusal = GlobalReadFixture::read_without_global_values();
        assert_eq!(refusal.code(), Some(&DiagnosticCode::MISSING_HOST_GROUP));
        assert!(
            refusal
                .labels()
                .map(|label| label.span())
                .eq([GlobalReadFixture::span()])
        );
    }

    #[test]
    fn the_refusal_of_a_global_read_says_what_was_refused() {
        let refusal = GlobalReadFixture::read_without_global_values();
        assert!(
            refusal.title().contains("read a global"),
            "{}",
            refusal.title()
        );
    }
}
