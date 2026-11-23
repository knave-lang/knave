use rustc_hash::FxHashMap;

use super::kept_temporary_problems::{KeptTemporary, KeptTemporaryProblems};
use super::promoted_constants::PromotedConstants;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::modules::ModuleSources;
use crate::types::{
    ParameterRole, Type, TypedArgument, TypedBinder, TypedBlock, TypedBodyItem, TypedControl,
    TypedDefinition, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf, TypedProgram,
};

pub struct KeptTemporaries<'p> {
    program: &'p TypedProgram,
    functions: FxHashMap<BindingId, &'p TypedLambda>,
    constants: PromotedConstants<'p>,
    problems: Vec<Diagnostic>,
}

impl<'p> KeptTemporaries<'p> {
    pub fn check(program: &'p TypedProgram) -> Vec<Diagnostic> {
        let mut check = Self {
            program,
            functions: Self::functions(program),
            constants: PromotedConstants::new(program),
            problems: Vec::new(),
        };
        for definition in Self::user_definitions(program) {
            check.walk(&definition.value);
        }
        check.problems
    }

    fn functions(program: &TypedProgram) -> FxHashMap<BindingId, &TypedLambda> {
        program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .filter_map(|definition| match &definition.value.kind {
                TypedExprKind::Fn(lambda) => Some((definition.binding, lambda.as_ref())),
                _ => None,
            })
            .collect()
    }

    fn user_definitions(program: &TypedProgram) -> impl Iterator<Item = &TypedDefinition> {
        program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.definitions)
    }

    fn walk(&mut self, expr: &'p TypedExpr) {
        match &expr.kind {
            TypedExprKind::Call(callee, arguments)
                if let TypedExprKind::Fn(body) = &callee.kind =>
            {
                self.bound_arguments(arguments);
                self.block(&body.body);
            }
            TypedExprKind::Recur(arguments) => self.bound_arguments(arguments),
            TypedExprKind::Fn(lambda) => {
                self.block(&lambda.body);
                self.returned(&lambda.body);
            }
            TypedExprKind::Control(TypedControl::Block(block)) => self.block(block),
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                self.walk(&matched.value);
                for arm in &matched.arms {
                    self.block(&arm.body);
                }
            }
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| self.walk(child)),
        }
    }

    fn bound_arguments(&mut self, arguments: &'p [TypedArgument]) {
        for argument in arguments {
            self.outliving(&argument.value);
            self.walk(&argument.value);
        }
    }

    fn block(&mut self, block: &'p TypedBlock) {
        for item in &block.items {
            match item {
                TypedBodyItem::Let(_, value) => {
                    self.outliving(value);
                    self.walk(value);
                }
                TypedBodyItem::Expr(expr) => self.walk(expr),
            }
        }
    }

    fn returned(&mut self, body: &'p TypedBlock) {
        if let Some(tail) = body.tail() {
            self.outliving(tail);
        }
    }

    fn outliving(&mut self, value: &'p TypedExpr) {
        match &value.kind {
            TypedExprKind::Control(TypedControl::Block(block)) => self.returned(block),
            TypedExprKind::Control(TypedControl::If(_, then, otherwise)) => {
                self.outliving(then);
                self.outliving(otherwise);
            }
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                for arm in &matched.arms {
                    self.returned(&arm.body);
                }
            }
            TypedExprKind::Call(callee, _) if let TypedExprKind::Fn(body) = &callee.kind => {
                self.returned(&body.body);
            }
            TypedExprKind::Call(callee, arguments)
                if let TypedExprKind::Leaf(TypedLeaf::Global(function)) = callee.kind =>
            {
                self.kept_call(function, arguments);
            }
            _ => {}
        }
    }

    fn kept_call(&mut self, function: BindingId, arguments: &'p [TypedArgument]) {
        let Some(lambda) = self.functions.get(&function).copied() else {
            return;
        };
        if !Self::returns_closure(lambda) {
            return;
        }
        let held = self
            .program
            .ownership
            .regions
            .of(ParameterRole::HeldByResult, lambda);
        for (parameter, argument) in lambda.params.iter().zip(arguments) {
            if held.contains(&parameter.binding) && self.lends_temporary(parameter, &argument.value)
            {
                self.report(function, parameter, &argument.value);
            }
        }
    }

    fn returns_closure(lambda: &TypedLambda) -> bool {
        lambda
            .body
            .tail()
            .is_some_and(|tail| matches!(tail.ty, Type::Function(..)))
    }

    fn lends_temporary(&self, parameter: &TypedBinder, argument: &TypedExpr) -> bool {
        let lent = match &argument.kind {
            TypedExprKind::Borrow(_, value) => value,
            _ => argument,
        };
        parameter.ty.is_reference() && self.constants.is_unpromoted_value(lent)
    }

    fn report(&mut self, function: BindingId, parameter: &TypedBinder, argument: &TypedExpr) {
        let bindings = &self.program.bindings;
        self.problems
            .push(KeptTemporaryProblems::kept_temporary(&KeptTemporary {
                argument: argument.span,
                callee: bindings.get(function).name(),
                parameter: bindings.get(parameter.binding).name(),
                parameter_span: parameter.span,
            }));
    }
}
