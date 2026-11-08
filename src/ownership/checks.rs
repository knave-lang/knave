use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::problems::{MacroFieldRead, OwnershipProblems};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::types::{
    Borrow, Pipelines, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram,
    TypedStructExpr,
};

pub struct OwnershipNeeds<'a> {
    program: &'a TypedProgram,
    function_params: &'a FxHashMap<BindingId, Vec<Type>>,
    pipelines: Pipelines<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> OwnershipNeeds<'a> {
    pub fn new(
        program: &'a TypedProgram,
        function_params: &'a FxHashMap<BindingId, Vec<Type>>,
        pipelines: Pipelines<'a>,
    ) -> Self {
        Self {
            program,
            function_params,
            pipelines,
            diagnostics: Vec::new(),
        }
    }

    pub fn check(mut self, binding: BindingId, value: &TypedExpr) -> Vec<Diagnostic> {
        let current = self.function_params.get(&binding).cloned();
        match &value.kind {
            TypedExprKind::Fn(_) => value
                .children()
                .into_iter()
                .for_each(|child| self.value(child, current.as_deref())),
            _ => self.value(value, current.as_deref()),
        }
        self.diagnostics
    }

    fn value(&mut self, expr: &TypedExpr, current: Option<&[Type]>) {
        match &expr.kind {
            TypedExprKind::Call(callee, arguments) => self.check_arguments(callee, arguments),
            TypedExprKind::Recur(arguments) => self.check_recur(arguments, current),
            _ => {}
        }
        let child_current = match expr.kind {
            TypedExprKind::Fn(_) => None,
            _ => current,
        };
        for child in expr.children() {
            self.value(child, child_current);
        }
    }

    fn check_arguments(&mut self, callee: &TypedExpr, arguments: &[TypedArgument]) {
        let parameters = self.parameters(callee);
        self.check_with_parameters(Some(callee), arguments, &parameters);
    }

    fn check_recur(&mut self, arguments: &[TypedArgument], parameters: Option<&[Type]>) {
        if let Some(parameters) = parameters {
            self.check_with_parameters(None, arguments, parameters);
        }
    }

    fn check_with_parameters(
        &mut self,
        callee: Option<&TypedExpr>,
        arguments: &[TypedArgument],
        parameters: &[Type],
    ) {
        for (position, argument) in arguments.iter().enumerate() {
            let owns = parameters
                .get(position)
                .is_some_and(|parameter| !parameter.is_reference());
            if argument.borrow == Borrow::Passthrough && owns {
                let diagnostic = self.needs_ownership(callee, &argument.value);
                self.diagnostics.push(diagnostic);
            }
        }
    }

    fn needs_ownership(&self, callee: Option<&TypedExpr>, value: &TypedExpr) -> Diagnostic {
        if self.pipelines.is_pipeline(value.ty.referent()) {
            return OwnershipProblems::needs_pipeline_ownership(value.span);
        }
        match self.macro_field_read(callee, value) {
            Some(read) => OwnershipProblems::macro_field_needs_ownership(&read),
            None => OwnershipProblems::needs_ownership(value.span),
        }
    }

    fn macro_field_read(
        &self,
        callee: Option<&TypedExpr>,
        value: &TypedExpr,
    ) -> Option<MacroFieldRead<'a>> {
        let TypedExprKind::Struct(TypedStructExpr::Field(_, label)) = &value.kind else {
            return None;
        };
        let callee = callee.filter(|callee| callee.span != value.span)?;
        let macro_name = self.program.macro_field_reads.get(&value.span)?;
        Some(MacroFieldRead {
            macro_name: Rc::clone(macro_name),
            field: Rc::clone(&label.name),
            field_span: label.span,
            field_type: self.program.show_type(value.ty.referent()),
            callee: self.function_name(callee),
        })
    }

    fn function_name(&self, callee: &TypedExpr) -> Option<&'a str> {
        match callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(function) | TypedLeaf::Local(function)) => {
                Some(self.program.bindings.get(function).name())
            }
            _ => None,
        }
    }

    fn parameters(&self, callee: &TypedExpr) -> Vec<Type> {
        if let TypedExprKind::Leaf(TypedLeaf::Global(function) | TypedLeaf::Local(function)) =
            callee.kind
            && let Some(parameters) = self.function_params.get(&function)
        {
            return parameters.clone();
        }
        match &callee.ty {
            Type::Function(_, parameters, _) => parameters.clone(),
            _ => Vec::new(),
        }
    }
}
