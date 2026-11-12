use super::callee_definitions::CalleeDefinitions;
use super::plan::EmitPlan;
use super::short_form::ShortForm;
use crate::source::{SourceId, Span};
use crate::types::{
    TypedArgument, TypedBodyItem, TypedExpr, TypedExprKind, TypedProgram, TypedSource,
};

pub struct TestProgram {
    source: &'static str,
    program: TypedProgram,
}

impl TestProgram {
    pub fn new(source: &'static str) -> Self {
        Self {
            source,
            program: TypedSource::expand(source).typed(),
        }
    }

    pub fn program(&self) -> &TypedProgram {
        &self.program
    }

    pub fn plan(&self) -> EmitPlan<'_> {
        EmitPlan::build(&self.program, Span::new(SourceId::new(0), 0..0))
            .unwrap_or_else(|_| panic!("the source defines main"))
    }

    pub fn short_form<'p>(plan: &'p EmitPlan<'p>) -> ShortForm<'p> {
        ShortForm::new(CalleeDefinitions::new(plan))
    }

    pub fn last_let_value(&self, function: &str) -> &TypedExpr {
        let body = self
            .program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .filter(|definition| self.program.bindings.get(definition.binding).name() == function)
            .find_map(|definition| match &definition.value.kind {
                TypedExprKind::Fn(lambda) => Some(&lambda.body),
                _ => None,
            })
            .expect("the source defines the function");
        body.items
            .iter()
            .rev()
            .find_map(|item| match item {
                TypedBodyItem::Let(_, value) => Some(value),
                TypedBodyItem::Expr(_) => None,
            })
            .expect("the function binds a local")
    }

    pub fn arguments(call: &TypedExpr) -> &[TypedArgument] {
        match &call.kind {
            TypedExprKind::Call(_, arguments) => arguments,
            _ => panic!("the expression is a call"),
        }
    }

    pub fn callee(call: &TypedExpr) -> &TypedExpr {
        match &call.kind {
            TypedExprKind::Call(callee, _) => callee,
            _ => panic!("the expression is a call"),
        }
    }

    pub fn text(&self, expr: &TypedExpr) -> &'static str {
        &self.source[expr.span.range()]
    }
}
