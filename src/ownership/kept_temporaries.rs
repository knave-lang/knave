use super::built_parts::BuiltParts;
use super::call_result_sources::CallResultSources;
use super::kept_temporary_problems::{KeptBorrow, KeptTemporary, KeptTemporaryProblems, KeptUntil};
use super::lent_temporary::{LentTemporary, LentValue};
use crate::diagnostic::Diagnostic;
use crate::modules::ModuleSources;
use crate::types::{
    CalledFunction, DefinitionTable, Regions, ReturnedValues, TypedArgument, TypedBinder,
    TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda, TypedMatch,
    TypedProgram,
};

pub struct KeptTemporaries<'p> {
    program: &'p TypedProgram,
    definitions: &'p DefinitionTable<'p>,
    regions: &'p Regions,
    temporaries: LentTemporary<'p>,
    problems: Vec<Diagnostic>,
}

struct OutlivingCall<'p> {
    called: CalledFunction<'p, 'p>,
    kept_until: KeptUntil,
}

impl<'p> KeptTemporaries<'p> {
    pub fn new(program: &'p TypedProgram, definitions: &'p DefinitionTable<'p>) -> Self {
        Self {
            program,
            definitions,
            regions: &program.ownership.regions,
            temporaries: LentTemporary::new(
                program,
                CallResultSources::new(program, definitions),
                BuiltParts::new(program, definitions),
            ),
            problems: Vec::new(),
        }
    }

    pub fn check(mut self) -> Vec<Diagnostic> {
        let user_definitions = self
            .program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.definitions);
        for definition in user_definitions {
            self.walk(&definition.value);
        }
        self.problems
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
            TypedExprKind::Fn(lambda) => self.lambda(lambda),
            TypedExprKind::Control(TypedControl::Block(block)) => self.block(block),
            TypedExprKind::Control(TypedControl::Match(matched)) => self.matched(matched),
            _ => expr
                .children()
                .into_iter()
                .for_each(|child| self.walk(child)),
        }
    }

    fn lambda(&mut self, lambda: &'p TypedLambda) {
        self.block(&lambda.body);
        if let Some(tail) = lambda.body.tail() {
            self.outliving(tail, KeptUntil::FunctionReturn);
        }
    }

    fn matched(&mut self, matched: &'p TypedMatch) {
        self.walk(&matched.value);
        for arm in &matched.arms {
            self.block(&arm.body);
        }
    }

    fn bound_arguments(&mut self, arguments: &'p [TypedArgument]) {
        for argument in arguments {
            self.outliving(&argument.value, KeptUntil::StatementEnd);
            self.walk(&argument.value);
        }
    }

    fn block(&mut self, block: &'p TypedBlock) {
        for item in &block.items {
            match item {
                TypedBodyItem::Let(_, value) => {
                    self.outliving(value, KeptUntil::StatementEnd);
                    self.walk(value);
                }
                TypedBodyItem::Expr(expr) => self.walk(expr),
            }
        }
    }

    fn outliving(&mut self, value: &'p TypedExpr, kept_until: KeptUntil) {
        for outliving in ReturnedValues::new(self.program).of_value(value) {
            if let Some(called) = self.definitions.called_function(outliving) {
                self.kept_call(OutlivingCall { called, kept_until });
            }
        }
    }

    fn kept_call(&mut self, call: OutlivingCall<'p>) {
        let lambda = call.called.lambda;
        if !lambda.returns_closure() {
            return;
        }
        let kept = self.regions.kept_by_result(lambda);
        for (parameter, argument) in lambda.params.iter().zip(call.called.arguments) {
            if !kept.contains(&parameter.binding) {
                continue;
            }
            if let Some(lent) = self.temporaries.lent_in(parameter, &argument.value) {
                self.report(&call, parameter, lent);
            }
        }
    }

    fn report(&mut self, call: &OutlivingCall, parameter: &TypedBinder, lent: LentValue) {
        let bindings = &self.program.bindings;
        self.problems
            .push(KeptTemporaryProblems::kept_temporary(&KeptTemporary {
                argument: lent.value.span,
                callee: bindings.get(call.called.function).name(),
                parameter: bindings.get(parameter.binding).name(),
                parameter_span: parameter.span,
                kept_borrow: KeptBorrow::of(&parameter.ty),
                kept_until: call.kept_until,
                kept_value: lent.kind,
            }));
    }
}
