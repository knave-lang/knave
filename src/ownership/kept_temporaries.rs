use super::kept_temporary_problems::{KeptTemporary, KeptTemporaryProblems, KeptUntil};
use super::promoted_constants::PromotedConstants;
use super::statement_temporary::StatementTemporary;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::modules::ModuleSources;
use crate::types::{
    DefinitionTable, Regions, ReturnedValues, TypedArgument, TypedBinder, TypedBlock,
    TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram,
};

pub struct KeptTemporaries<'p> {
    program: &'p TypedProgram,
    definitions: &'p DefinitionTable<'p>,
    regions: &'p Regions,
    constants: PromotedConstants<'p>,
    problems: Vec<Diagnostic>,
}

struct OutlivingCall<'e> {
    function: BindingId,
    arguments: &'e [TypedArgument],
    kept_until: KeptUntil,
}

impl<'p> KeptTemporaries<'p> {
    pub fn new(
        program: &'p TypedProgram,
        definitions: &'p DefinitionTable<'p>,
        regions: &'p Regions,
    ) -> Self {
        Self {
            program,
            definitions,
            regions,
            constants: PromotedConstants::new(program),
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
            TypedExprKind::Fn(lambda) => {
                self.block(&lambda.body);
                if let Some(tail) = lambda.body.tail() {
                    self.outliving(tail, KeptUntil::FunctionReturn);
                }
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
            if let TypedExprKind::Call(callee, arguments) = &outliving.kind
                && let TypedExprKind::Leaf(TypedLeaf::Global(function)) = callee.kind
            {
                self.kept_call(OutlivingCall {
                    function,
                    arguments,
                    kept_until,
                });
            }
        }
    }

    fn kept_call(&mut self, call: OutlivingCall<'p>) {
        let Some(lambda) = self
            .definitions
            .definition(call.function)
            .and_then(|definition| definition.value.lambda_value())
        else {
            return;
        };
        if !lambda.returns_closure() {
            return;
        }
        let kept = self.regions.kept_by_result(lambda);
        for (parameter, argument) in lambda.params.iter().zip(call.arguments) {
            if kept.contains(&parameter.binding) && parameter.ty.is_reference() {
                self.temporary_argument(&call, parameter, &argument.value);
            }
        }
    }

    fn temporary_argument(
        &mut self,
        call: &OutlivingCall,
        parameter: &TypedBinder,
        argument: &TypedExpr,
    ) {
        if StatementTemporary::of(argument, self.constants).is_none() {
            return;
        }
        let bindings = &self.program.bindings;
        self.problems
            .push(KeptTemporaryProblems::kept_temporary(&KeptTemporary {
                argument: argument.span,
                callee: bindings.get(call.function).name(),
                parameter: bindings.get(parameter.binding).name(),
                parameter_span: parameter.span,
                kept_until: call.kept_until,
            }));
    }
}
